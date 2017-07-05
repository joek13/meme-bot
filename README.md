# MemeBot
MemeBot is a Discord bot that can help you and your users make memes. Using a set of owner-defined templates, users can use the `+meme` command to generate memes at will. 

## Installing/Running
------

Clone this repo and run with `cargo run`. Be sure that you have set the env var `DISCORD_TOKEN` (i.e. `export DISCORD_TOKEN=mytoken`) to your bot's token before running.

## Templates
------

Templates are TOML files loaded from the `./templates/` directory and provide a description of all the content that goes into a meme. Templates start with the required fields `name`, `short_name`, and `image`. `name` and `short_name` help identify the template, but only `short_name` is used to actually invoke the template. `image` is a path to the base image to add to (relative to the template file itself). After that, *features* are listed. *Features* are parts of a template that can be filled in by users, and are what allow the bot to have unique content generated. Features can be either `text` or `image` features. `text` features act as simple text-boxes, whereas `image` features are areas for images to be pasted on. **All** features use the `x`, `y`, `w`, and `h` properties (as well as `rotation`, optionally) to define the rectangle that text or images can be overlaid within. More specific properties for `kind="Text"` features are as follows:

 * `font_size` - fairly self explanatory. The height of a single line (in pixels). Required.
 * `font_color` - a 4-element array of integers 0-255 representing the font's color (in order: red, green, blue, and alpha). Optional, but defaults to black.
 * `alignment` - either `Left`, `Center`, or `Right`. Defaults to center.


 Some `kind="Image"` specific properties:

  * `stretch` - whether to stretch the image (warping the aspect ratio) to make it fit the target rect. Optional, and defaults to false.

## Commands
------

Users can use `+list` to list all the available templates. `+meme <template> <text/image> ...` will generate a meme with template `template` and the provided text. `+info <template>` will give more specific information on any one template.
