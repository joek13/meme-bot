# MemeBot
MemeBot is a Discord bot that can help you and your users make memes. Using a set of owner-defined templates, users can use the `+meme` command to generate memes at will. 

## Installing/Running
------

Clone this repo and run with `cargo run`. Be sure that you have set the env var `DISCORD_TOKEN` (i.e. `export DISCORD_TOKEN=mytoken`) to your bot's token before running.
## Configuring
------

The bot looks for a file called `config.toml` in your current working directory, and won't run without it. `config.toml` should look something like `example_config.toml`. For your convenience, you can rename `example_config.toml` to `config.toml` and then fill in your token, etc.
## Templates
------

Templates are TOML files loaded from the `./templates/` directory and provide a description of all the content that goes into a meme. Templates start with the required fields `kind`, `name`, `short_name`, and `image`. `name` and `short_name` help identify the template, but only `short_name` is used to actually invoke the template. `image` is a path to the base image to add to (relative to the template file itself). After that, *features* are listed. *Features* are parts of a template that can be filled in by users, and are what allow the bot to have unique content generated. Features can be either `text` or `image` features. `text` features act as simple text-boxes, whereas `image` features are areas for images to be pasted on. **All** features use the `x`, `y`, `w`, and `h` properties (as well as `rotation`, optionally) to define the rectangle that text or images can be overlaid within. More specific properties for `kind="Text"` features are as follows:

 * `font_size` - fairly self explanatory. The height of a single line (in pixels). Required.
 * `font_color` - a 4-element array of integers 0-255 representing the font's color (in order: red, green, blue, and alpha). Optional, but defaults to black.
 * `alignment` - either `Left`, `Center`, or `Right`. Defaults to center.


 Some `kind="Image"` specific properties:

  * `stretch` - whether to stretch the image (warping the aspect ratio) to make it fit the target rect. Optional, and defaults to false.
## Template Manifest Spec
------
| Property | Type |Required | Description                                |
|:--------:|:----:|:--------:|--------------------------------------------|
| `name`     | String | Required | The long, descriptive name to show alongside generated images. |
| `short_name`| String | Required | The short, easy name to use with commands. |
| `image` | Path String | Required | The base image to build templates from. The path is relative to this template. |
| `features` | List | Required | A list of features to put on the template. |

### Feature Dict Spec
A feature is a TOML dictionary. Each feature can have the following properties:

| Property | Type |Required | Description                                |
|:--------:|:----:|:--------:|--------------------------------------------|
| `kind` | String | Required | The type of feature this is. Can be `Text`, `Image`, or `Either`. |
| `x` | int | Required | The x-coordinate of the top-left corner of this feature, in pixels. |
| `y` | int | Required | The y-coordinate of the top-left corner of this feature, in pixels. |
| `w` | int | Required | The width of this feature, in pixels. |
| `h` | int | Required | The height of this feature, in pixels. |
| `rotation` | float | Optional | The rotation of this feature, in degrees. Features are rotated around their top-left corner. |

#### Image-specific properties
These properties are specific to `Image` and `Either` features, and will be ignored in `Text` features.

| Property | Type |Required | Description                                |
|:--------:|:----:|:--------:|--------------------------------------------|
| `stretch` | bool | Optional | Whether to stretch the target image to fit the provided rect. Stretching will *not* maintain the image's aspect ratio. Defaults to `false`. |
| `mask` | Path String | Optional | A path to a mask image. Mask images are grayscale, and *must* match the dimensions of the template image. The mask will be applied to this feature only, and parts of the mask that are not white will cause those parts of the feature be masked out in generated images. Leaving this off will result in no masking.
#### Text-specific properties
These properties are specific to `Text` and `Either` features, and will be ignored in `Image` features.
| Property | Type |Required | Description                                |
|:--------:|:----:|:--------:|--------------------------------------------|
| `font_size` | int | Required | The maximum font size to use in generated images, in pixels. |
| `alignment` | String | Optional | The text alignment to use. Defaults to `Left`, but can be `Left`, `Center`, or `Right`. |
| `font_color` | [int, int, int, int] | Optional | An array four integers 0-255 long representing the font color to use. Channels are R, G, B, A. Defaults to [0, 0, 0, 255]. |

#### Example template
`whowouldwin.toml` from `./templates`: 
```
name="Who Would Win"
short_name="whowouldwin"
image="./whowouldwin.png"
[[features]]
kind="Image" #the first image
x=20
y=90
w=216
h=216
[[features]]
kind="Text" #its caption
x=20
y=311
w=216
h=43
font_size=43
alignment="Center"

[[features]]
kind="Image" #the second image
x=276
y=90
w=216
h=216
[[features]]
kind="Text" #its caption
x=276
y=311
w=216
h=43
font_size=43
alignment="Center"
```
## Commands
------
From the output of `+help`:
```
  help - Lists all the commands or gives specific help for one command..
  tip - Replies with a pro-tip for using the bot.
  info - Gets more specific information about a template.
  list - Lists all the templates to choose from.
  meme - Generates an image based on a template.
  invite - Replies with a link to invite me to your server.
```
