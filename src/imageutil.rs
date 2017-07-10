use image::GenericImage;
use image;
use image::imageops::resize;
use image::Pixel;
use image::GrayImage;
use image::RgbaImage;
use image::Rgba;
use std::cmp::min;

pub fn paste_image<D: GenericImage + 'static, S: GenericImage<Pixel = D::Pixel> + 'static>(
    source: &S,
    destination: &mut D,
    x: u32,
    y: u32,
) {
    for i in 0..source.width() {
        for k in 0..source.height() {
            if i + x < destination.width() && k + y < destination.height() {
                let pixel = source.get_pixel(i, k);
                let mut other_pixel = destination.get_pixel(i + x, k + y);
                other_pixel.blend(&pixel);
                destination.put_pixel(i + x, k + y, other_pixel);
            }
        }
    }
}
pub fn paste_image_resized<
    D: GenericImage + 'static,
    S: GenericImage<Pixel = D::Pixel> + 'static,
>(
    source: &S,
    destination: &mut D,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) {
    let resized = resize(source, w, h, image::FilterType::Nearest); //resize image

    for i in 0..resized.width() {
        for k in 0..resized.height() {
            if i + x < destination.width() && k + y < destination.height() {
                let pixel = resized.get_pixel(i, k);
                let mut other_pixel = destination.get_pixel(i + x, k + y);
                other_pixel.blend(pixel);
                destination.put_pixel(i + x, k + y, other_pixel);
            }
        }
    }
}
pub fn mask_image(input_image: RgbaImage, mask_image: &GrayImage) -> RgbaImage {
    assert_eq!(input_image.width(), mask_image.width());
    assert_eq!(input_image.height(), mask_image.height());
    let mut output_image = RgbaImage::new(input_image.width(), input_image.height());
    for i in 0..mask_image.width() {
        for k in 0..mask_image.height() {
            let mask_pixel = mask_image.get_pixel(i, k);
            let input_pixel = input_image.get_pixel(i, k);
            let new_pixel = Rgba {
                data: [
                    input_pixel.data[0],
                    input_pixel.data[1],
                    input_pixel.data[2],
                    min(mask_pixel.data[0], input_pixel.data[3]),
                ], //don't use mask alpha if the original image's alpha is less
            };
            output_image.put_pixel(i, k, new_pixel)
        }
    }
    output_image
}
