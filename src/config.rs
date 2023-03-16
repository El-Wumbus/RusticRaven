use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config
{
    /// Where Markdown source files are stored
    pub source: PathBuf,

    /// Where Html source file are stored
    pub dest: PathBuf,

    /// Where sublime syntax highliting files are stored
    pub syntaxes: PathBuf,

    /// One of the following themes:
    ///
    /// base16-ocean.dark
    /// base16-eighties.dark
    /// base16-mocha.dark
    /// base16-ocean.light
    /// InspiredGitHub
    /// Solarized (dark)
    /// Solarized (light)
    ///
    /// Or one found in the `custom_syntax_themes` dir.
    pub syntax_theme: String,

    /// Where `.tmTheme` color shemes are stored
    pub custom_syntax_themes: PathBuf,
}

impl Default for Config
{
    fn default() -> Self
    {
        Self {
            source:               PathBuf::from(Self::DEF_SRC_DIR),
            dest:                 PathBuf::from(Self::DEF_DEST_DIR),
            syntaxes:             PathBuf::from(Self::DEF_SYNTAXES_DIR),
            syntax_theme:         String::from(Self::DEF_SYNTAX_THEME),
            custom_syntax_themes: PathBuf::from(Self::DEF_CUSTOM_SYNTAX_THEMES_DIR),
        }
    }
}

impl Config
{
    pub const DEF_CONFIG_FILE: &str = "raven.toml";
    const DEF_CUSTOM_SYNTAX_THEMES_DIR: &str = "syntax-themes";
    const DEF_DEST_DIR: &str = "dest";
    const DEF_SRC_DIR: &str = "src";
    const DEF_SYNTAXES_DIR: &str = "syntaxes";
    const DEF_SYNTAX_THEME: &str = "base16-eighties.dark";

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
