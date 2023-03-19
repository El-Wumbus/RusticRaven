use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error
{
    #[error("[{}] IoError: \"{path}\": {err}", crate::NAME)]
    Io
    {
        err:  std::io::Error,
        path: std::path::PathBuf,
    },

    #[error("[{}] ConfigParseError: {0}", crate::NAME)]
    ConfigParse(String),

    #[error("[{}] SyntaxHighlightError: {0}", crate::NAME)]
    SyntaxHighlight(String),

    #[error("[{}] MissingPageInfoError: \"{0}\": Missing page info in file", crate::NAME)]
    MissingPageInfo(PathBuf),

    #[error("[{}] ParsePageInfoError: \"{path}\": {err}", crate::NAME)]
    ParsePageInfo
    {
        err: String, path: PathBuf
    },

    #[error("[{}] LoadSyntaxError: \"{path}\": {err}", crate::NAME)]
    LoadSyntax
    {
        err: String, path: PathBuf
    },

    #[error("[{}] LoadSyntaxThemesError: \"{path}\": {err}", crate::NAME)]
    LoadSyntaxThemes
    {
        err: String, path: PathBuf
    },

    #[error("[{}] ReadSourceDirError: \"{path}\": {err}", crate::NAME)]
    ReadSourceDir
    {
        err: String, path: PathBuf
    },

    #[error("[{}] MissingSourceFilesError: \"{0}\": No source files found", crate::NAME)]
    MissingSourceFiles(PathBuf),

    #[error(
        "[{}] MissingFaviconError: \"{source_file}\": Requested favicon file \"{expected_favicon_file}\", but it \
         doesn't exist",
        crate::NAME
    )]
    MissingFavicon
    {
        source_file:           PathBuf,
        expected_favicon_file: PathBuf,
    },

    #[error(
        "[{}] MissingTemplateError: \"{source_file}\": Requested favicon file \"{expected_template_file}\", but it \
         doesn't exist",
        crate::NAME
    )]
    MissingTemplate
    {
        source_file:            PathBuf,
        expected_template_file: PathBuf,
    },

    #[error(
        "[{}] MissingThemeError: Requested theme \"{0}\" in configuration file, but it doesn't exist",
        crate::NAME
    )]
    MissingTheme(String),

    #[error("[{}] HtmlPostprocessError: There was an error generated HTML: \"{0}\"", crate::NAME)]
    HtmlPostprocess(String),

    #[error(
        "[{}] AsyncJoinError: There was an internal error during the build process.",
        crate::NAME
    )]
    AysncJoin,

    #[error("[{}] IntegraionIntoTemplateError", crate::NAME)]
    IntegraionIntoTemplate,

    #[error("[{}] ProgressBarInitializationError", crate::NAME)]
    ProgressBarInitialization,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error
{
    pub fn unwrap_gracefully<T>(x: Result<T>) -> T
    {
        match x {
            Ok(value) => value,
            Err(e) => e.report_and_exit(),
        }
    }

    /// Prints the error and exits with the appropriate code
    pub fn report_and_exit(&self) -> !
    {
        let code = match self {
            Error::Io { .. } => 74,
            Error::ConfigParse(_) => 78,
            _ => 64,
        };
        eprintln!("{self}");
        std::process::exit(code);
    }

    pub fn report(&self)
    {
        eprintln!("{self}");
    }
}
