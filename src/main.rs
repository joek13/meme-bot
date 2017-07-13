extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate image;
extern crate imageproc;
extern crate rusttype;
extern crate textwrap;
#[macro_use]
extern crate serenity;
#[macro_use]
extern crate lazy_static;
extern crate time;
extern crate hyper;
extern crate url;
extern crate futures;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate hyper_native_tls;
extern crate regex;
extern crate rand;

mod template;
mod parse;
mod imageutil;
mod config;

use template::Template;

use config::Config;

use lazy_static::LazyStatic;

use regex::Regex;
use rand::Rng;

use std::fs;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::Arc;

use serenity::client;
use serenity::Client;
use serenity::model::Mentionable;
use serenity::framework::CommandGroup;
use serenity::model::Message;
use serenity::model::Ready;
use serenity::model::UserId;
use serenity::model::Game;
use serenity::client::Context;
use serenity::CACHE;

lazy_static! {
    static ref TEMPLATES: RwLock<Vec<Template>> = RwLock::new(Vec::new());
    static ref CONFIG: RwLock<Config> = RwLock::new(Config::new());
}
struct Handler {}
impl client::EventHandler for Handler {
    fn on_ready(&self, _ctx: Context, ready: Ready) {
        info!("Logged in as {}", ready.user.name);
        _ctx.set_game(Game::playing(
            format!(
                "{}help for help",
                CONFIG.read().unwrap().prefixes[0]
            ).as_str(),
        ));
    }
}
fn main() {
    env_logger::init().unwrap();
    info!("Loading config...");
    match Config::load_from("config.toml") {
        Err(e) => {
            error!("Error while loading config: {}", e.to_string());
        }
        Ok(config) => {
            let token;
            let prefixes;
            {
                let mut conf = CONFIG.write().unwrap();
                token = Some(config.token.clone());
                prefixes = Some(config.prefixes.clone());
                *conf = config;
            }
            info!("Loading templates...");
            LazyStatic::initialize(&TEMPLATES);
            match load_templates() {
                Ok(templates) => {
                    {
                        let mut cache = TEMPLATES.write().unwrap();
                        *cache = templates;
                    }
                    info!("Logging in...");
                    let mut client = Client::new(token.unwrap().as_str(), Handler {});
                    client.with_framework(move |f| {
                        f.configure(|c| c.prefixes(prefixes.unwrap().iter().map(|x| x.as_str()).collect()))
                            .command("meme", |c| {
                                c.exec(meme)
                                    .desc("Generates an image based on a template.")
                                    .example("<template> \"text 1\" \"text 2\" ...")
                            })
                            .command("help", |c| {
                                c.exec_help(help)
                                    .desc("Lists all the commands or gives specific help for one command..")
                            })
                            .command("list",
                                     |c| c.exec(list).desc("Lists all the templates to choose from."))
                            .command("info", |c| {
                                c.exec(info)
                                    .desc("Gets more specific information about a template.")
                                    .example("<template>")
                            })
                            .command("invite", |c| {
                                c.exec(invite)
                                    .desc("Replies with a link to invite me to your server.")
                            })
                            .command("prefix", |c| {
                                c.exec(prefix)
                                    .desc("List all the prefixes you can reach the bot with.")
                                    .known_as("prefixes")
                            })
                            .command("tip", |c| {
                                c.exec(tip)
                                    .desc("Replies with a pro-tip for using the bot.")
                            })
                    });
                    let _ = client.start();
                }
                Err((template, e)) => {
                    if let Some(filename) = template {
                        error!("Error loading template {}: {}", filename, e);
                    } else {
                        error!("Error loading templates: {}", e);
                    }
                }
            }
        }
    }
}
fn get_template<'a>(templates: &'a Vec<Template>, name: &str) -> Option<&'a Template> {
    templates.iter().find(|template| {
        template.short_name == name || template.aliases.contains(&name.to_owned())
    })
}
fn load_templates() -> Result<Vec<Template>, (Option<String>, template::Error)> {
    let mut templates = Vec::new();
    let files = fs::read_dir("./templates").map_err(|e| {
        (None, template::Error::Io(e))
    })?;
    for file in files {
        let path = file.map_err(|e| (None, template::Error::Io(e)))?.path();
        match path.extension().map(|e| e.to_str().unwrap_or("")) {
            Some("toml") => {
                templates.push(Template::from_file(path.as_path()).map_err(|e| {
                    (Some(path.to_str().unwrap().to_owned()), e)
                })?);
            }
            _ => {}
        }
    }
    Ok(templates)
}
fn list_templates() -> String {
    TEMPLATES
        .read()
        .unwrap()
        .iter()
        .map(|x| format!("`{}`", x.short_name))
        .collect::<Vec<String>>()
        .join(", ")
}
command!(meme(_ctx, message, args) {
    match args.len() {
        0|1 => {
            let ref prefix = CONFIG.read().unwrap().prefixes[0];
            let _ = message.reply(format!("**Usage**: `{}meme <template> \"<text1>\" \"[text2]\" ...`\nTemplates you can use: {}\nUse `{}info <template>` for more specific information.", prefix, list_templates(), prefix).as_str());
        }
        _ => {
            let ref template_name = args[0];
            let templates = TEMPLATES.read().unwrap();
            let template = get_template(&templates, template_name.as_str());
            if let Some(template) = template {
                let texts = args.iter().skip(1).map(|x| x.as_str()).collect::<Vec<&str>>();
                match parse::parse_text(texts.as_slice()) {
                    Ok(mut texts) => {
                            let mention_regex = Regex::new("^<@!?([0-9]+)>$").unwrap();
                            let mut replacements: Vec<(usize, &serenity::model::User)> = Vec::new();
                            for (text_index, text) in texts.iter().enumerate() {
                                if let Some(captures) = mention_regex.captures(text) {
                                    if let Ok(id) = captures.get(1).unwrap().as_str().parse::<u64>() {
                                        for mention in &message.mentions {
                                            if mention.id == id {
                                                //we've found the user!
                                                replacements.push((text_index, mention));
                                            }
                                        }
                                    }
                                }
                            }
                            for (index, user) in replacements {
                                //replace the mention with avatar url
                                let _ = texts.remove(index);
                                let avatar_url = if let Some(url) = user.avatar_url() {url} else {user.default_avatar_url()};
                                let avatar_url = avatar_url.replace(".webp", ".png"); //hacky, but image doesn't support webp properly
                                texts.insert(index, avatar_url); //insert it back
                            }
                            match template.render(texts.iter().map(|x| x.as_str()).collect::<Vec<&str>>().as_slice(), false) {
                                Ok(image) => {
                                    let mut buf: Vec<u8> = Vec::new();
                                    let _ = image.save(&mut buf, image::ImageFormat::PNG);
                                    let _ = message.channel_id.send_files(vec![(buf.as_slice(), "meme.png")], |m|
                                                                          m.content(
                                                                              format!("**{}**", template.name).as_str()));
                                },
                                Err(e) => {
                                    warn!("Error rendering: {}", e);
                                    let _ = message.reply(e.to_string().as_str());
                                }
                            }
                    }
                    Err(e) => {
                        let _ = message.reply(format!("Error parsing your input: {}", e.to_string()).as_str());
                    }
                }
            } else {
                let _ = message.reply(format!("{} is not a valid template. Options: {}", template_name, list_templates()).as_str());
            }
        }
    }
});
command!(list(_ctx, message) {
    let _ = message.channel_id.say(format!("Hi {}, here's a list of all the templates you can use: {}\nUse **{}info <meme>** to get more specific information.", message.author.mention(), list_templates(), CONFIG.read().unwrap().prefixes[0]).as_str());
});
const TIPS: &[&'static str] = &[
    "Wanna make fun of your friends? @-mention them in lieu of an image, and the resulting meme will have their avatar!",
    "If you put too much text in a text box, it will automatically be sized down until it fits.",
    "Use the `info` command to get the down-n-dirty details about a template.",
    "Text with spaces in it needs to be escaped with quotes (\"). If your argument is a url, or the text is only one word, then leave the quotes out!",
    "If you want quotes inside your meme, escape them with a backslash (\\\\\"). If you want to use a backslash, just escape it with another one!",
    "Both double quotes (\") and single quotes (\') can be used to have spaces in text. Since only the outermost kind of quote is recognized, single quotes can be used unescaped inside of double quotes and vice-versa.",
    "If you insta-pick Jungle Legion, you're trash.",
];
command!(tip(_ctx, message) {
    let index = rand::thread_rng().gen_range::<usize>(0, TIPS.len());
    let _ = message.channel_id.say(format!("***Pro Memester Tip #{}:*** {}", index+1, TIPS[index]).as_str());
});
command!(info(_ctx, message, args) {
    match args.len() {
        0 => {
            let _ = message.reply("Provide the name of the meme you want more information about.");
        }
        _ => {
            let ref template = args[0];
            if let Some(template) = get_template(&TEMPLATES.read().unwrap(), template.as_str()) {
                let mut texts = Vec::new();
                for i in 0..template.features.len() {
                    texts.push(format!("Text {}", i+1));
                }
                let mut example_usage = format!("{}meme {} ", CONFIG.read().unwrap().prefixes[0], template.short_name);
                for feature in &template.features {
                    use template::FeatureType;
                    match feature.kind {
                        FeatureType::Image => {
                            example_usage += "\"<image>\" ";
                        }
                        FeatureType::Either => {
                            example_usage += "\"<text/image>\"";
                        }
                        FeatureType::Text => {
                            example_usage += "\"<text>\" ";
                        }
                    }
                }
                let image = template.render(texts.iter().map(|x| x.as_str()).collect::<Vec<&str>>().as_slice(), true).unwrap();
                let mut buf = Vec::new();
                let _ = image.save(&mut buf, image::ImageFormat::PNG);
                let filename = "meme.png";
                //show info
                let _ = message.channel_id.send_files(vec![(buf.as_slice(), filename)], |m|
                    m.content(
                        format!("**{}**\n**Short name**: {}\n**Aliases:** {}\n**Features:** {}\n**Example Usage:** `{}`\n**Template:**",
                                template.name,
                                template.short_name,
                                if template.aliases.len() > 0 {template.aliases.join(", ")} else {"None".to_owned()},
                                template.features.len(), 
                                example_usage)
                        .as_str()
                    ));
            } else {
                let _ = message.reply(format!("Template `{}` not found. Use `{}list` for a list of templates.", template, CONFIG.read().unwrap().prefixes[0]).as_str());
            }
        }
    }
});
fn invite_url(id: UserId) -> String {
    format!(
        "https://discordapp.com/oauth2/authorize?permissions=35840&scope=bot&client_id={}",
        id
    )
}
command!(prefix(_ctx, message) {
    let ref prefixes = CONFIG.read().unwrap().prefixes;
    let prefixes = prefixes.iter().map(|x| format!("`{}`", x.trim())).collect::<Vec<String>>().join(", ");
    let _ = message.reply(format!("Prefixes you can reach me with: {}", prefixes).as_str());
});
command!(invite(_ctx, message) {
    let _ = message.reply(format!("Use this link to invite me to your server: {}", invite_url(CACHE.read().unwrap().user.id)).as_str());
});
fn help(
    _ctx: &mut Context,
    message: &Message,
    commands: HashMap<String, Arc<CommandGroup>>,
    args: &[String],
) -> Result<(), String> {
    match args.len() {
        0 => {
            let mut response = format!(
                "Hello {}, here's a list of commands:\n",
                message.author.mention()
            );
            //list all commands
            for group in commands.values() {
                for (name, command) in &group.commands {
                    use serenity::framework::CommandOrAlias;
                    if let CommandOrAlias::Command(ref command) = *command {
                        let description = {
                            if let Some(ref desc) = command.desc {
                                desc.as_str()
                            } else {
                                "No description provided"
                            }
                        };
                        response += format!("  **{}** - {}\n", name, description).as_str();
                    }
                }
            }
            response += format!(
                "Use `{}help <command>` to get more specific information about one command.",
                CONFIG.read().unwrap().prefixes[0]
            ).as_str();
            let _ = message.channel_id.say(response.as_str());
        }
        1 => {
            //info for one command
            let ref name = args[0];
            let mut command_found = false;
            for group in commands.values() {
                for (command_name, command) in &group.commands {
                    if command_name == &name.to_lowercase() {
                        use serenity::framework::CommandOrAlias;
                        if let CommandOrAlias::Command(ref command) = *command {
                            let description = {
                                if let Some(ref desc) = command.desc {
                                    desc.as_str()
                                } else {
                                    "No description provided"
                                }
                            };
                            let example = {
                                if let Some(ref example) = command.example {
                                    format!(
                                        "{}{} {}",
                                        CONFIG.read().unwrap().prefixes[0],
                                        command_name,
                                        example
                                    )
                                } else {
                                    format!(
                                        "{}{}",
                                        CONFIG.read().unwrap().prefixes[0],
                                        command_name
                                    )
                                }
                            };
                            let _ = message
                                .channel_id
                                .say(format!("**Info for command {}:**\n  **Description:** {}\n  **Example usage:** `{}`",
                                     command_name,
                                     description,
                                     example).as_str());
                        }
                        command_found = true;
                        break;
                    }
                }
            }
            if !command_found {
                let _ = message.reply(
                    format!(
                        "Command `{}` not found. Type `{}help` to list commands.",
                        name,
                        CONFIG.read().unwrap().prefixes[0]
                    ).as_str(),
                );
            }
        }
        _ => {
            let _ = message.reply("Too many arguments");
            return Err("Too many arguments".to_owned());
        }
    }
    Ok(())
}
