use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use structstruck::strike;

use crate::{Error, Result};


strike! {
    #[strikethrough[derive(Debug, Clone, Deserialize, Serialize)]]
    pub struct Config
    {
        /// Markdown source files
        pub source: PathBuf,

        /// Where generated HTML files
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

        pub default: pub struct Defaults {
            /// The default favicon for webpages.
            pub favicon: PathBuf,

            /// The default css stylesheet for webpages.
            pub stylesheet: PathBuf,

            /// The default HTML template for webpages.
            pub template: PathBuf,
        },

        pub generation: Option<pub struct Generation {
            /// If generated HTML should be processed (minimized, etc.)
            pub process: Option<pub struct ProcessHtml {
                pub minify: bool,
            }>,

            /// Treat html found in the source directory as a template
            pub treat_source_as_template: Option<bool>,
        }>,
    }
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
            custom_syntax_themes: PathBuf::from(Self::DEFAULT_CUSTOM_SYNTAX_THEMES_DIR),
            generation:           None,
            default:              Defaults {
                favicon:    PathBuf::from(Self::DEFAULT_FAVICON_FILE),
                stylesheet: PathBuf::from(Self::DEFUALT_STYLE_FILE),
                template:   PathBuf::from(Self::DEFAULT_TEMPLATE_FILE),
            },
        }
    }
}

impl Config
{
    pub const DEFAULT_CONFIG_FILE: &str = "raven.toml";
    const DEFAULT_CUSTOM_SYNTAX_THEMES_DIR: &str = "syntax-themes";
    const DEFAULT_DEST_DIR: &str = "dest";
    const DEFAULT_FAVICON_FILE: &str = "favicon.ico";
    const DEFAULT_SRC_DIR: &str = "src";
    const DEFAULT_SYNTAXES_DIR: &str = "syntaxes";
    const DEFAULT_SYNTAX_THEME: &str = "base16-eighties.dark";
    const DEFAULT_TEMPLATE_FILE: &str = "template.html";
    const DEFUALT_STYLE_FILE: &str = "style.css";

    /// Constructs a `Config` from a TOML file provided (`path`).
    ///
    /// # Errors
    ///
    /// Will return an error if:
    ///
    /// - The `path` cannot be read into a string
    /// - The TOML read from `path` cannot be parsed into a `Config`
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
