use std::{borrow::Cow, ffi::OsString, fs, io::Write, path::PathBuf, rc::Rc};

use gh_emoji::Replacer;
use pulldown_cmark::{CodeBlockKind, Event};
use serde::Deserialize;
use structopt::StructOpt;
use syntect::{highlighting, parsing::SyntaxSet};

mod error;
pub use error::*;

mod config;
pub use config::*;
use walkdir::DirEntry;

const NAME: &str = "RusticRaven";
const DESC: &str = "A static html generator";

const TEMPLATE_NAME_BODY: &str = "//<rustic_body>//";
const TEMPLATE_NAME_TITLE: &str = "//<rustic_title>//";
const TEMPLATE_NAME_DESC: &str = "//<rustic_description>//";
const TEMPLATE_NAME_FAVICON: &str = "//<rustic_favicon>//";

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
        #[structopt(long = "config", default_value = Config::DEF_CONFIG_FILE)]
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
    const DEFAULT_FAVICON_PATH: &str = "favicon.png";
}

fn init(directory: &PathBuf)
{
    let config = Config::default();
    let configuration_file_path = directory.join(Config::DEF_CONFIG_FILE);

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

fn build(config: Config, directory: PathBuf) -> Result<()>
{
    use walkdir::WalkDir;

    let (syntax_set_builder, mut themes) = get_syntaxes(&config, &directory)?;
    let theme = themes.remove(&config.syntax_theme).unwrap();
    let site = Rc::new(Website::new(syntax_set_builder.build(), theme, &config));
    let source_dir = directory.join(&config.source);
    // Walk the source directory and filter the results to only include files
    // that have a markdown file extention
    let source_file_dir: Vec<DirEntry> = WalkDir::new(&source_dir)
        .into_iter()
        .filter_map(|x| {
            dbg!(&x);
            let x = x;
            if x.is_ok() && {
                let extention: &str = &x
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .unwrap_or(&OsString::new())
                    .to_string_lossy()
                    .to_lowercase();

                x.as_ref().unwrap().path().is_file() && (extention == "markdown" || extention == "md")
            } {
                Some(x.unwrap())
            }
            else {
                // If x is an error we print an error, but we continue.
                if x.is_err() {
                    let e = Error::ReadSourceDir {
                        err:  x.as_ref().err().unwrap().to_string(),
                        path: PathBuf::from(x.unwrap().path()),
                    };
                    e.report();
                }
                None
            }
        })
        .collect();

    let source_file_count = source_file_dir.len();

    // If there's no source files we exit with an error
    if source_file_count == 0 {
        Error::MissingSourceFiles(source_dir).report_and_exit();
    }

    for source_file in source_file_dir {
        let source_path = source_file.path().to_path_buf();
        let source_file_name = source_path.file_stem().unwrap();
        let dest_path: PathBuf =
            directory.join(config.dest.join(format!("{}.html", source_file_name.to_string_lossy())));

        // Parse the markdown into html
        let (html, page_info) = Error::unwrap_gracefully(site.clone().parse_markdown(source_path.clone()));
        let template = directory.join(page_info.template);

        // If the template file doesn't exist, skip this file
        if !template.is_file() {
            Error::MissingTemplate {
                source_file:            source_path,
                expected_template_file: template,
            }
            .report();
            continue;
        }

        // Get the favicon file path
        let favicon = directory.join(
            page_info
                .favicon
                .unwrap_or(PathBuf::from(PageInfo::DEFAULT_FAVICON_PATH)),
        );

        // If the favicon file doesn't exist, skip this file.
        if !favicon.is_file() {
            Error::MissingFavicon {
                source_file:           source_path,
                expected_favicon_file: favicon,
            }
            .report();
            continue;
        }

        // Add the markdown html into the template html, then write it out.
        let html = Error::unwrap_gracefully(fs::read_to_string(&template).map_err(|e| {
            Error::Io {
                err:  e,
                path: template.clone(),
            }
        }))
        .replace(TEMPLATE_NAME_BODY, &html)
        .replace(TEMPLATE_NAME_TITLE, &page_info.title)
        .replace(TEMPLATE_NAME_DESC, &page_info.description)
        .replace(TEMPLATE_NAME_FAVICON, &favicon.to_string_lossy());

        Error::unwrap_gracefully(fs::write(&dest_path, html).map_err(|e| {
            Error::Io {
                err:  e,
                path: dest_path,
            }
        }));
    }

    Ok(())
}

fn get_syntaxes(
    config: &Config,
    directory: &PathBuf,
) -> Result<(
    syntect::parsing::SyntaxSetBuilder,
    std::collections::BTreeMap<String, highlighting::Theme>,
)>
{
    let syntax_dir = directory.join(&config.syntaxes);
    let custom_syntax_themes_dir = directory.join(&config.custom_syntax_themes);

    let mut syntax_set_builder = SyntaxSet::load_defaults_newlines().into_builder();
    if syntax_dir.is_dir() {
        syntax_set_builder.add_from_folder(&syntax_dir, true).map_err(|e| {
            let e = Error::LoadSyntax {
                path: syntax_dir,
                err:  e.to_string(),
            };

            // Report the error and exit if this fails
            e.report_and_exit()
        })?;
    }

    let mut themes = highlighting::ThemeSet::load_defaults().themes;
    if custom_syntax_themes_dir.is_dir() {
        let custom_theme_files =
            highlighting::ThemeSet::discover_theme_paths(&custom_syntax_themes_dir).map_err(|e| {
                Error::LoadSyntaxThemes {
                    err:  e.to_string(),
                    path: custom_syntax_themes_dir.clone(),
                }
                .report_and_exit()
            })?;


        // Get the custom themes and add them to the theme map.
        for custom_theme_file in custom_theme_files {
            let theme = highlighting::ThemeSet::get_theme(&custom_theme_file).map_err(|e| {
                Error::LoadSyntaxThemes {
                    err:  e.to_string(),
                    path: custom_syntax_themes_dir.clone(),
                }
                .report_and_exit()
            })?;

            let name = theme.name.clone().unwrap_or(
                custom_theme_file
                    .file_stem()
                    .unwrap_or(&OsString::from("unknown"))
                    .to_string_lossy()
                    .to_string(),
            );

            // Add the custom theme to the theme list
            themes.insert(name, theme);
        }
    }
    Ok((syntax_set_builder, themes))
}


struct Website
{
    emoji_replacer: Replacer,
    syntax_set:     SyntaxSet,
    syntax_theme:   highlighting::Theme,
    config:         Config,
}

impl Website
{
    fn new(syntax_set: SyntaxSet, syntax_theme: highlighting::Theme, config: &Config) -> Self
    {
        Self {
            emoji_replacer: Replacer::new(),
            syntax_set,
            syntax_theme,
            config: config.clone(),
        }
    }

    fn parse_markdown(&self, source_path: PathBuf) -> Result<(String, PageInfo)>
    {
        let source = fs::read_to_string(&source_path).map_err(|e| {
            Error::Io {
                err:  e,
                path: source_path.clone(),
            }
        })?;

        use pulldown_cmark::{html, Options, Parser, Tag};

        // Enable features that aren't part of the standard, but are widely
        // used.
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(&source, options);

        let mut html_out = String::new();
        let mut current_language = None;
        let mut unparsed_page_info = None;
        let mut markdown_html = Vec::new();
        'next_event: for mut event in parser {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                    current_language = Some(lang.clone());

                    if lang.as_ref() == PageInfo::CODE_BLOCK_IDENTIFIER || lang.as_ref().starts_with("rustic") {
                        continue 'next_event;
                    }
                }
                Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                    current_language = None;

                    // Suppress templateinfo stuff
                    // Suppress templateinfo and handler stuff
                    if lang.as_ref() == PageInfo::CODE_BLOCK_IDENTIFIER || lang.as_ref().starts_with("rustic") {
                        continue 'next_event;
                    }
                }
                Event::Text(ref mut text) => {
                    // Insert emojis
                    if let Cow::Owned(new_text) = self.emoji_replacer.replace_all(text) {
                        *text = new_text.into();
                    }

                    if let Some(lang) = current_language.as_ref() {
                        if lang.as_ref() == PageInfo::CODE_BLOCK_IDENTIFIER {
                            unparsed_page_info = Some(text.to_string());
                            continue 'next_event;
                        }
                        else if let Some(syntax) = self.syntax_set.find_syntax_by_token(lang) {
                            let highlighted_html = match syntect::html::highlighted_html_for_string(
                                text,
                                &self.syntax_set,
                                syntax,
                                &self.syntax_theme,
                            ) {
                                Ok(x) => x,
                                Err(e) => return Err(Error::SyntaxHighlight(e.to_string())),
                            };

                            // Change the event to an html event
                            event = Event::Html(highlighted_html.into())
                        }
                    }
                }
                _ => {}
            }

            markdown_html.push(event);
        }

        // Parse the markdown to HTML
        html::push_html(&mut html_out, markdown_html.into_iter());

        let unparsed_page_info = unparsed_page_info.ok_or_else(|| Error::MissingPageInfo(source_path.clone()))?;
        let page_info = toml::from_str::<PageInfo>(&unparsed_page_info).map_err(|e| {
            Error::ParsePageInfo {
                err:  e.to_string(),
                path: source_path,
            }
        })?;
        Ok((html_out, page_info))
    }
}
