use std::{
    borrow::Cow,
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    rc::Rc,
};

use gh_emoji::Replacer;
use pulldown_cmark::{CodeBlockKind, Event};
use serde::Deserialize;
use structopt::StructOpt;
use syntect::{highlighting, parsing::SyntaxSet};

mod error;
pub use error::*;

mod config;
pub use config::*;

mod build;
use build::*;

const NAME: &str = "RusticRaven";
const DESC: &str = "A static html generator";

const TEMPLATE_NAME_BODY: &str = "[/rustic_body/]";
const TEMPLATE_NAME_TITLE: &str = "[/rustic_title/]";
const TEMPLATE_NAME_DESC: &str = "[/rustic_description/]";
const TEMPLATE_NAME_FAVICON: &str = "[/rustic_favicon/]";

#[derive(Debug, StructOpt)]
#[structopt(
    name = NAME,
    about = DESC,
)]
enum Options
{
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

fn main() -> error::Result<()>
{
    let options = Options::from_args();
    match &options {
        Options::Init { directory } => init(directory),
        Options::Build { config_path, directory } => {
            Error::unwrap_gracefully(build(
                Error::unwrap_gracefully(Config::from_toml(&directory.join(config_path))),
                directory.to_path_buf(),
            ))
        }
    };


    Ok(())
}

fn init(directory: &PathBuf)
{
    let config = Config::default();
    let configuration_file_path = directory.join(Config::DEFAULT_CONFIG_FILE);

    if configuration_file_path.exists() {
        return;
    }

    // Open a new conf file, we err if the file already exists
    let mut f = Error::unwrap_gracefully(fs::File::create(&configuration_file_path).map_err(|e| {
        Error::Io {
            err:  e,
            path: configuration_file_path.clone(),
        }
    }));
    // Serialize the defualt values, then write it to the new config file;
    let toml = toml::to_string_pretty(&config).unwrap();
    Error::unwrap_gracefully(f.write_all(toml.as_bytes()).map_err(|e| {
        Error::Io {
            err:  e,
            path: configuration_file_path,
        }
    }));

    // create dirs
    let source = directory.join(&config.source);
    let dest = directory.join(&config.dest);
    let syntaxes = directory.join(&config.syntaxes);
    let custom_syntax_themes = directory.join(&config.custom_syntax_themes);
    Error::unwrap_gracefully(fs::create_dir(&source).map_err(|e| Error::Io { err: e, path: source }));
    Error::unwrap_gracefully(fs::create_dir(&dest).map_err(|e| Error::Io { err: e, path: dest }));
    Error::unwrap_gracefully(fs::create_dir(&syntaxes).map_err(|e| {
        Error::Io {
            err:  e,
            path: syntaxes,
        }
    }));
    Error::unwrap_gracefully(fs::create_dir(&custom_syntax_themes).map_err(|e| {
        Error::Io {
            err:  e,
            path: custom_syntax_themes,
        }
    }));
}
