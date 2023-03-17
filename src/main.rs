use std::{
    borrow::Cow,
    ffi::OsString,
    io::Write,
    path::{Path, PathBuf},
};

use gh_emoji::Replacer;
use indicatif::{ProgressIterator, ProgressStyle};
use pulldown_cmark::{CodeBlockKind, Event};
use serde::Deserialize;
use structopt::StructOpt;
use syntect::{highlighting, parsing::SyntaxSet};
use tokio::fs;
use walkdir::{DirEntry, WalkDir};

mod error;
pub use error::*;
mod config;
pub use config::*;
mod build;
use build::*;
mod defaults;

const NAME: &str = "RusticRaven";
const DESC: &str = "A static html generator";

const TEMPLATE_NAME_BODY: &str = "[/rustic_body/]";
const TEMPLATE_NAME_TITLE: &str = "[/rustic_title/]";
const TEMPLATE_NAME_DESC: &str = "[/rustic_description/]";
const TEMPLATE_NAME_FAVICON: &str = "[/rustic_favicon/]";
const TEMPLATE_NAME_STYLESHEET: &str = "[/rustic_stylesheet/]";

#[derive(Debug, StructOpt)]
#[structopt(
    name = NAME,
    about = DESC,
)]
enum Options
{
    /// Create a new directory and initalize it
    New
    {
        /// The name of the new project
        name: PathBuf,

        /// The name of the source directory
        #[structopt(short = "s", long = "source")]
        source: Option<String>,

        /// The name of the output directory (Where the generated HTML goes).
        #[structopt(short = "d", long = "dest")]
        dest: Option<String>,

        /// The name of the custom syntax directory.
        #[structopt(short = "y", long = "syntaxes")]
        syntaxes: Option<String>,

        /// The name of the custom syntax themes directory
        #[structopt(short = "t", long = "syntax_themes")]
        syntax_themes: Option<String>,
    },

    /// Initialize a new project
    Init
    {
        /// The project directory
        #[structopt(default_value = ".")]
        directory: PathBuf,
    },

    /// Build static HTML from an existing project
    Build
    {
        /// The project directory
        #[structopt(default_value = ".")]
        directory: PathBuf,

        /// Provide an alternate config file path
        #[structopt(long = "config", default_value = Config::DEFAULT_CONFIG_FILE)]
        config_path: PathBuf,

        /// Rebuild all file regardless of if the sources have been modified
        #[structopt(long = "rebuild_all", short = "a")]
        rebuild_all: bool,
    },

    /// Clean the dest dir of generated files and directories
    Clean
    {
        /// The project directory
        #[structopt(default_value = ".")]
        directory: PathBuf,

        /// Provide an alternate config file path
        #[structopt(long = "config", default_value = Config::DEFAULT_CONFIG_FILE)]
        config_path: PathBuf,
    },
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PageInfo
{
    /// The page title.
    title: String,

    /// The page's description.
    description: String,

    /// The CSS stylesheet to use.
    style: PathBuf,

    /// The path to the HTML template to use.
    template: PathBuf,

    /// Use a different favicon for this page. If omitted the defualt one will
    /// be used.
    favicon: Option<PathBuf>,
}

impl PageInfo
{
    const CODE_BLOCK_IDENTIFIER: &str = "pageinfo";
}

#[tokio::main]
async fn main() -> error::Result<()>
{
    let options = Options::from_args();
    let initial_directory = Error::unwrap_gracefully(PathBuf::from(".").canonicalize().map_err(|e| {
        Error::Io {
            err:  e,
            path: PathBuf::from("."),
        }
    }));

    match &options {
        Options::Init { directory } => {
            // Change directories into the specified directory.
            std::env::set_current_dir(directory).unwrap();
            Error::unwrap_gracefully(init(Config::default()).await)
        }
        Options::Build {
            config_path,
            directory,
            rebuild_all,
        } => {
            // Change directories into the specified directory.
            std::env::set_current_dir(directory).unwrap();
            let config = Error::unwrap_gracefully(Config::from_toml(config_path));
            let (syntax_set_builder, mut themes) = Error::unwrap_gracefully(get_syntaxes(&config));
            let theme = match themes.remove(&config.syntax_theme) {
                None => Err(Error::MissingTheme(config.syntax_theme.clone())),
                Some(x) => Ok(x),
            }?;
            let site = Website::new(config, syntax_set_builder.build(), theme);
            Error::unwrap_gracefully(build(site, *rebuild_all).await)
        }
        Options::Clean { directory, config_path } => {
            // Change directories into the specified directory.
            std::env::set_current_dir(directory).unwrap();
            Error::unwrap_gracefully(clean(Error::unwrap_gracefully(Config::from_toml(config_path))).await)
        }
        Options::New {
            name,
            source,
            dest,
            syntaxes,
            syntax_themes,
        } => {
            let mut config = Config::default();
            // Create the name dir
            if let Err(e) = fs::create_dir_all(name).await {
                Error::Io {
                    err:  e,
                    path: name.to_path_buf(),
                }
                .report_and_exit()
            }

            if let Some(source) = source {
                let source = PathBuf::from(source);
                config.source = source;
            }
            if let Some(dest) = dest {
                let dest = PathBuf::from(dest);
                config.dest = dest;
            }
            if let Some(syntaxes) = syntaxes {
                let syntaxes = PathBuf::from(syntaxes);
                config.dest = syntaxes;
            }
            if let Some(syntax_themes) = syntax_themes {
                let syntax_themes = PathBuf::from(syntax_themes);
                config.dest = syntax_themes;
            }
            // Change directories into the specified directory.
            std::env::set_current_dir(name).unwrap();
            Error::unwrap_gracefully(init(config).await)
        }
    };

    // Change directories back into the inital directory.
    std::env::set_current_dir(initial_directory).unwrap();

    Ok(())
}

async fn clean(config: Config) -> Result<()>
{
    let pbs = ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
        .unwrap()
        .progress_chars("#>-");

    let dest_dir = &config.dest;
    if dest_dir.is_dir()
        && dest_dir
            .read_dir()
            .map_err(|e| {
                Error::Io {
                    err:  e,
                    path: dest_dir.clone(),
                }
            })?
            .next()
            .is_none()
    {
        return Ok(());
    }
    let dest_dir_contents: Vec<DirEntry> = WalkDir::new(dest_dir)
        .into_iter()
        .filter_map(|x| {
            if let Ok(x) = x {
                if x.path() != dest_dir {
                    Some(x)
                }
                else {
                    None
                }
            }
            else {
                None
            }
        })
        .collect();

    // We delete all the files inside the dest dir and create a progress bar to
    // track the progress.
    for path in dest_dir_contents.iter().progress_with_style(pbs) {
        let path = path.path();
        if path.is_file() {
            fs::remove_file(path).await.map_err(|e| {
                Error::Io {
                    err:  e,
                    path: path.to_path_buf(),
                }
            })?;
        }
        else if path.is_dir() {
            fs::remove_dir_all(path).await.map_err(|e| {
                Error::Io {
                    err:  e,
                    path: path.to_path_buf(),
                }
            })?;
        }
    }
    Ok(())
}

/// Initialize a directiory with the defualt doodads
async fn init(config: Config) -> Result<()>
{
    let configuration_file_path = PathBuf::from(Config::DEFAULT_CONFIG_FILE);

    if configuration_file_path.exists() {
        return Ok(());
    }

    // Open a new conf file, we err if the file already exists
    let f = fs::File::create(&configuration_file_path).await.map_err(|e| {
        Error::Io {
            err:  e,
            path: configuration_file_path.clone(),
        }
    })?;
    println!("Created: \"{}\"", configuration_file_path.display());

    // Serialize the defualt values, then write it to the new config file;
    let toml = toml::to_string_pretty(&config).unwrap();
    f.into_std().await.write_all(toml.as_bytes()).map_err(|e| {
        Error::Io {
            err:  e,
            path: configuration_file_path,
        }
    })?;

    // create dirs
    let source = config.source;
    let dest = config.dest;
    let syntaxes = config.syntaxes;
    let custom_syntax_themes = config.custom_syntax_themes;
    fs::create_dir(&source).await.map_err(|e| {
        Error::Io {
            err:  e,
            path: source.clone(),
        }
    })?;
    println!("Created: \"{}\"", source.display());
    fs::create_dir(&dest).await.map_err(|e| {
        Error::Io {
            err:  e,
            path: dest.clone(),
        }
    })?;
    println!("Created: \"{}\"", dest.display());
    fs::create_dir(&syntaxes).await.map_err(|e| {
        Error::Io {
            err:  e,
            path: syntaxes.clone(),
        }
    })?;
    println!("Created: \"{}\"", syntaxes.display());
    fs::create_dir(&custom_syntax_themes).await.map_err(|e| {
        Error::Io {
            err:  e,
            path: custom_syntax_themes.clone(),
        }
    })?;
    println!("Created: \"{}\"", custom_syntax_themes.display());
    fs::write("template.html", defaults::DEFAULT_HTML_TEMPLATE_SRC)
        .await
        .map_err(|e| {
            Error::Io {
                err:  e,
                path: PathBuf::from("template.html"),
            }
        })?;
    println!("Created: \"template.html\"");
    fs::write("style.css", defaults::DEFAULT_CSS_STYLESHEET_SRC)
        .await
        .map_err(|e| {
            Error::Io {
                err:  e,
                path: PathBuf::from("style.css"),
            }
        })?;
    println!("Created: \"style.css\"");
    fs::write("src/index.md", defaults::DEFAULT_MD_STARTER_SRC)
        .await
        .map_err(|e| {
            Error::Io {
                err:  e,
                path: PathBuf::from("src/index.md"),
            }
        })?;
    println!("Created: \"src/index.md\"");
    Ok(())
}
