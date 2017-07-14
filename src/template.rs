use std::path::{Path, PathBuf};
use std::io::Read;
use std::fs::File;
use std::borrow::ToOwned;
use std::iter;
use std::f32::consts::PI;

pub use self::error::{Result, Error};

use imageutil::*;

use toml;

use hyper::client::Client;
use hyper::net::HttpsConnector;

use url::Url;

use textwrap::wrap;

use image::{DynamicImage, GenericImage, RgbaImage};
use image::Rgba;
use image;

use rusttype::{FontCollection, Font, Scale};

use imageproc::drawing::{draw_text_mut, draw_hollow_rect_mut};
use imageproc::rect;
use imageproc::affine::rotate_with_default;
use imageproc::affine::Interpolation;

use hyper_native_tls::NativeTlsClient;

const FONT: &[u8] = include_bytes!("Roboto.ttf");
const DEG_2_RAD: f32 = PI / 180.0;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Template {
    pub image: PathBuf,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub short_name: String,
    pub features: Vec<Feature>,
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Eq, PartialEq)]
pub enum FeatureType {
    Text,
    Image,
    Either,
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Feature {
    pub kind: FeatureType,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub rotation: Option<f32>,
    pub font_size: Option<f32>,
    pub font_color: Option<[u8; 4]>,
    pub alignment: Option<Alignment>,
    pub stretch: Option<bool>,
    pub mask: Option<PathBuf>,
    #[serde(default)]
    pub margin_left: u32,
    #[serde(default)]
    pub margin_right: u32,
    #[serde(default)]
    pub margin_top: u32,
    #[serde(default)]
    pub margin_bottom: u32,
}
impl Template {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Template> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut template: Template = toml::from_str(contents.as_str())?;
        template.image = path.parent().unwrap_or(path).join(&template.image);
        let bg_image_dim;
        //make sure the image exists, is valid, and cache the dimensions in case there are any
        //masks.
        match image::open(&template.image) {
            Ok(image) => {
                //gotta get dimensions to check the masks
                bg_image_dim = image.dimensions();
            }
            Err(e) => {
                return Err(Error::Invalid(
                    format!("Error loading background image: {}", e.to_string()),
                ));
            }
        }
        for feature in &mut template.features {
            if feature.kind == FeatureType::Text || feature.kind == FeatureType::Either {
                if let None = feature.font_size {
                    return Err(Error::Invalid(
                        "Text feature is missing required field 'font_size'"
                            .to_owned(),
                    ));
                }
                if let None = feature.font_color {
                    feature.font_color = Some([0, 0, 0, 255]); //default to black
                }
                if feature.margin_left + feature.margin_right > feature.w {
                    return Err(Error::Invalid(
                        "Horizontal margins add up to more than feature's width"
                            .to_owned(),
                    ));
                }
                if feature.margin_top + feature.margin_bottom > feature.h {
                    return Err(Error::Invalid(
                        "Vertical margins add up to more than feature's height"
                            .to_owned(),
                    ));
                }
            }
            if feature.kind == FeatureType::Image || feature.kind == FeatureType::Either {
                if let Some(ref mut mask_path) = feature.mask {
                    let relative = path.parent().unwrap_or(path).join(&mask_path);
                    if !relative.exists() {
                        return Err(Error::Invalid("Image mask doesn't exist".to_owned()));
                    }
                    //check that mask is valid image, as well as its dimensions matching
                    match image::open(&relative) {
                        Ok(img) => {
                            if img.dimensions() != bg_image_dim {
                                return Err(Error::Invalid(
                                    "Mask dimensions do not match background image dimensions"
                                        .to_string(),
                                ));
                            }
                        }
                        Err(e) => {
                            return Err(Error::Invalid(format!(
                                "Error opening mask image {}: {}",
                                relative.to_string_lossy(),
                                e.to_string()
                            )));
                        }

                    }
                    *mask_path = relative;
                }
            }
        }
        Ok(template)
    }
    fn generate_text_image(
        feature: &Feature,
        bg_image: &DynamicImage,
        font: &Font,
        show_rectangles: bool,
        text: &str,
    ) -> Result<RgbaImage> {
        assert!(feature.kind == FeatureType::Text || feature.kind == FeatureType::Either);
        let mut font_image = RgbaImage::new(bg_image.width(), bg_image.height());
        if show_rectangles {
            //for debug and templates
            draw_hollow_rect_mut(
                &mut font_image,
                rect::Rect::at(feature.x as i32, feature.y as i32).of_size(feature.w, feature.h),
                Rgba([255, 0, 0, 255]),
            );
        }
        let mut height = feature.font_size.unwrap();
        let mut scale = Scale {
            x: height,
            y: height,
        };
        //rest of the calculations have to use the rect with margin factored in
        let feature_rect = Rect::new(
            feature.x + feature.margin_left, //offset left edge by margin_left
            feature.y + feature.margin_top, //offset top edge by margin_top
            feature.w - (feature.margin_left + feature.margin_right), //width = width - margins
            feature.h - (feature.margin_top + feature.margin_bottom), //height = height - margins
        );
        let mut max_lines = (feature_rect.h as f32 / height).floor() as usize;
        let mut char_width = (feature_rect.w as f32 * 2.4 / height).floor() as usize; //Magic Number (tm) to get char width from rect width
        while wrap(text, char_width).len() > max_lines {
            height -= 1.0;
            scale = Scale {
                x: height,
                y: height,
            };
            max_lines = (feature_rect.h as f32 / height).floor() as usize;
            char_width = (feature_rect.w as f32 * 2.4 / height).floor() as usize; //Magic Number (tm)
        }
        for (line_index, line) in
            align_text(
                text,
                char_width,
                feature.alignment.unwrap_or(Alignment::Left),
            ).iter()
                .enumerate()
        {
            if line_index >= max_lines {
                break;
            }
            draw_text_mut(
                &mut font_image,
                Rgba(feature.font_color.unwrap()),
                feature_rect.x,
                feature_rect.y + (line_index as f32 * height) as u32,
                scale,
                &font,
                line,
            );
        }
        if let Some(rotation) = feature.rotation {
            font_image = rotate_with_default(
                &font_image,
                (feature.x as f32, feature.y as f32),
                rotation * DEG_2_RAD,
                Rgba([0, 0, 0, 0]),
                Interpolation::Bilinear,
            );
        }
        //masking: mask the font_image with the mask bitmap (if given)
        if let Some(ref path) = feature.mask {
            let mask = image::open(path)?;
            font_image = mask_image(font_image, &mask.to_luma());
        }
        Ok(font_image)
    }
    fn generate_image_image(
        feature: &Feature,
        bg_image: &DynamicImage,
        url: &str,
    ) -> Result<RgbaImage> {
        let mut image = Vec::new();
        if let Ok(url) = Url::parse(url) {
            let ssl = NativeTlsClient::new().unwrap();
            let connector = HttpsConnector::new(ssl);
            let client = Client::with_connector(connector);
            let resp = client.get(url.clone()).send();
            match resp {
                Ok(mut resp) => {
                    resp.read_to_end(&mut image)?;
                }
                Err(e) => {
                    println!("error in url {}: {}", url, e);
                    let mut placeholder = File::open("./placeholder.png")?;
                    placeholder.read_to_end(&mut image)?;
                }
            }
        } else {
            let mut placeholder = File::open("./placeholder.png")?;
            placeholder.read_to_end(&mut image)?;
        }
        let mut underlay_image = RgbaImage::new(bg_image.width(), bg_image.height());

        let overlay_image = image::load_from_memory(image.as_slice())?;
        let bg_aspect = feature.w as f32 / feature.h as f32;
        let mut dim = overlay_image.dimensions();
        let mut offset = (0u32, 0u32);
        if !feature.stretch.unwrap_or(false) {
            let aspect = dim.0 as f32 / dim.1 as f32; //width over height
            if aspect > bg_aspect {
                //width > height
                dim.0 = feature.w;
                dim.1 = (1.0 / aspect * feature.w as f32) as u32;
                offset.1 = ((feature.h - dim.1) as f32 / 2.0) as u32;
            } else {
                //height > width or height = width
                dim.1 = feature.h;
                dim.0 = (aspect * feature.h as f32) as u32;
                offset.0 = ((feature.w - dim.0) as f32 / 2.0) as u32;
            }
        } else {
            dim = (feature.w, feature.h);
        }
        paste_image_resized(
            &overlay_image,
            &mut underlay_image,
            feature.x + offset.0,
            feature.y + offset.1,
            dim.0,
            dim.1,
        );

        if let Some(rotation) = feature.rotation {
            underlay_image = rotate_with_default(
                &underlay_image,
                (feature.x as f32, feature.y as f32),
                rotation * DEG_2_RAD,
                Rgba([0, 0, 0, 0]),
                Interpolation::Bilinear,
            );
        }
        //masking: mask the underlay_image with the mask bitmap (if given)
        if let Some(ref path) = feature.mask {
            let mask = image::open(path)?;
            underlay_image = mask_image(underlay_image, &mask.to_luma());
        }

        Ok(underlay_image)
    }
    pub fn render(&self, text: &[&str], show_rectangles: bool) -> Result<DynamicImage> {
        //load image
        let mut bg_image = image::open(&self.image)?;
        if self.features.len() == 0 {
            return Ok(bg_image); //no need to render any more
        }
        let font = Vec::from(FONT);
        let font = FontCollection::from_bytes(font).into_font().unwrap();
        for (index, feature) in self.features.iter().enumerate() {
            if index >= text.len() {
                break; //no text provided, leave blank
            } else {
                match feature.kind {
                    FeatureType::Text => {
                        let font_image = Template::generate_text_image(
                            feature,
                            &bg_image,
                            &font,
                            show_rectangles,
                            text[index],
                        )?;
                        paste_image(&font_image, &mut bg_image, 0, 0);
                    }
                    FeatureType::Image => {
                        let underlay_image =
                            Template::generate_image_image(feature, &bg_image, text[index])?;
                        paste_image(&underlay_image, &mut bg_image, 0, 0);
                    }
                    FeatureType::Either => {
                        //decide whether it is an image or a text
                        let image;
                        if let Ok(_) = Url::parse(text[index]) {
                            //it's an image!
                            image =
                                Template::generate_image_image(feature, &bg_image, text[index])?;
                        } else {
                            //it's text.
                            image = Template::generate_text_image(
                                feature,
                                &bg_image,
                                &font,
                                show_rectangles,
                                text[index],
                            )?;
                        }
                        paste_image(&image, &mut bg_image, 0, 0);
                    }
                }
            }
        }
        Ok(bg_image)
    }
}
struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}
impl Rect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect {
            x: x,
            y: y,
            w: w,
            h: h,
        }
    }
}
const PAD_CHAR: char = ' '; //this is actually an en space, which is significantly wider when rendered
fn align_text(text: &str, char_width: usize, alignment: Alignment) -> Vec<String> {
    let text = wrap(text, char_width);
    let mut padded = Vec::new();
    for line in text.into_iter() {
        let mut padding = 0;
        match alignment {
            Alignment::Center => {
                padding = ((char_width - line.chars().count()) as f32 / 2.0).floor() as usize;
            }
            Alignment::Right => {
                padding = char_width - line.chars().count();
            }
            _ => {}
        }
        let padded_line = iter::repeat(PAD_CHAR).take(padding).collect::<String>() + line.as_str();
        padded.push(padded_line);
    }
    padded
}
mod error {
    use std::result;
    use std::error;
    use std::io;
    use std::fmt;

    use toml;

    use image;

    pub type Result<T> = result::Result<T, self::Error>;

    #[derive(Debug)]
    pub enum Error {
        Io(io::Error),
        Deserialize(toml::de::Error),
        Invalid(String),
        Image(image::ImageError),
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let message = match *self {
                Error::Io(ref e) => e.to_string(),
                Error::Deserialize(ref e) => e.to_string(),
                Error::Invalid(ref message) => message.clone(),
                Error::Image(ref e) => e.to_string(),
            };
            write!(f, "{}", message)
        }
    }
    impl error::Error for Error {
        fn description(&self) -> &str {
            match *self {
                Error::Io(ref e) => e.description(),
                Error::Deserialize(ref e) => e.description(),
                Error::Invalid(_) => {
                    "The template was successfully deserialized, but contained invalid data."
                }
                Error::Image(ref e) => e.description(),
            }
        }
    }
    impl From<io::Error> for Error {
        fn from(e: io::Error) -> Error {
            Error::Io(e)
        }
    }
    impl From<toml::de::Error> for Error {
        fn from(e: toml::de::Error) -> Error {
            Error::Deserialize(e)
        }
    }
    impl From<image::ImageError> for Error {
        fn from(e: image::ImageError) -> Error {
            Error::Image(e)
        }
    }
}
