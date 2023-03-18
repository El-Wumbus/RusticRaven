use std::{path::PathBuf, sync::Arc};

use build::*;
use dashmap::DashMap;
use indicatif::{ProgressIterator, ProgressStyle};
pub use rustic_raven::*;
use structopt::StructOpt;
use tokio::fs;
use walkdir::{DirEntry, WalkDir};


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
            // The assets we've already loaded.
            // We use an Arc<DashMap> over an Arc<Mutex<Hashmap>> for finer-grained locking.
            // The changes are syncronized.
            let open_assets: Arc<DashMap<PathBuf, String>> = Arc::new(DashMap::new());
            let site = Website::new(config, syntax_set_builder.build(), open_assets, theme);
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
