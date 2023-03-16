use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config
{
    /// Where Markdown source files are stored
    pub source: PathBuf,

    /// Where generated HTML files are stored
    pub dest: PathBuf,

    /// Where sublime syntax highliting files are stored
    pub syntaxes: PathBuf,

    /// One of the following themes:
    ///
    /// `base16-ocean.dark`  
    /// `base16-eighties.dark`  
    /// `base16-mocha.dark`  
    /// `base16-ocean.light`  
    /// `InspiredGitHub`  
    /// `Solarized (dark)`  
    /// `Solarized (light)`  
    ///
    /// Or one found in the `custom_syntax_themes` dir.
    pub syntax_theme: String,

    /// Where `.tmTheme` color shemes are stored
    pub custom_syntax_themes: PathBuf,

    /// The default favicon for webpages.
    pub default_favicon: PathBuf,
}

impl Default for Config
{
    fn default() -> Self
    {
        Self {
            dest:                 PathBuf::from(Self::DEFAULT_DEST_DIR),
            source:               PathBuf::from(Self::DEFAULT_SRC_DIR),
            syntaxes:             PathBuf::from(Self::DEFAULT_SYNTAXES_DIR),
            syntax_theme:         String::from(Self::DEFAULT_SYNTAX_THEME),
            default_favicon:      PathBuf::from(Self::DEFAULT_FAVICON_FILE),
            custom_syntax_themes: PathBuf::from(Self::DEFAULT_CUSTOM_SYNTAX_THEMES_DIR),
        }
    }
}

impl Config
{
    pub const DEFAULT_CONFIG_FILE: &str = "raven.toml";
    const DEFAULT_CUSTOM_SYNTAX_THEMES_DIR: &str = "syntax-themes";
    const DEFAULT_DEST_DIR: &str = "dest";
    const DEFAULT_FAVICON_FILE: &str = "favicon.png";
    const DEFAULT_SRC_DIR: &str = "src";
    const DEFAULT_SYNTAXES_DIR: &str = "syntaxes";
    const DEFAULT_SYNTAX_THEME: &str = "base16-eighties.dark";

    pub fn from_toml(path: &PathBuf) -> Result<Self>
    {
        let contents = match fs::read_to_string(path) {
            Ok(x) => x,
            Err(e) => {
                return Err(Error::Io {
                    err:  e,
                    path: path.clone(),
                })
            }
        };

        let parsed = match toml::from_str(&contents) {
            Ok(x) => x,
            Err(e) => return Err(Error::ConfigParse(format!("Couldn't parse {}: {e}", path.display()))),
        };

        Ok(parsed)
    }
}
