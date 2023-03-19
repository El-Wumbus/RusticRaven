use std::path::{Path, PathBuf};

use serde::Deserialize;

pub mod build;
pub mod config;
pub mod defaults;
pub mod error;
pub use config::*;
pub use error::*;

pub const NAME: &str = "RusticRaven";
pub const DESC: &str = "A static html generator";

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PageInfo
{
    /// The page title.
    pub title: String,

    /// The page's description.
    pub description: String,

    /// The CSS stylesheet to use.
    pub style: Option<PathBuf>,

    /// The path to the HTML template to use.
    pub template: Option<PathBuf>,

    /// Use a different favicon for this page. If omitted the defualt one will
    /// be used.
    pub favicon: Option<PathBuf>,
}

impl PageInfo
{
    pub const CODE_BLOCK_IDENTIFIER: &str = "pageinfo";
}

/// Initialize a directiory with the defualt doodads
pub async fn init(config: Config) -> Result<()>
{
    use std::io::Write;

    use tokio::fs;
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
