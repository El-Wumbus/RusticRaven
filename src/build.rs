use std::{borrow::Cow, ffi::OsString, sync::Arc};

use chrono::{DateTime, Local};
use dashmap::DashMap;
use gh_emoji::Replacer;
use indicatif::ProgressStyle;
use pulldown_cmark::{CodeBlockKind, Event};
use syntect::{highlighting, parsing::SyntaxSet};
use tokio::fs;
use walkdir::WalkDir;

use crate::{Config, Error, PageInfo, Path, PathBuf, Result};

const TEMPLATE_NAME_BODY: &str = "[/rustic_body/]";
const TEMPLATE_NAME_TITLE: &str = "[/rustic_title/]";
const TEMPLATE_NAME_DESC: &str = "[/rustic_description/]";
const TEMPLATE_NAME_FAVICON: &str = "[/rustic_favicon/]";
const TEMPLATE_NAME_STYLESHEET: &str = "[/rustic_stylesheet/]";
const TEMPLATE_NAME_SITENAME: &str = "[/rustic_name/]";
const TEMPLATE_NAME_AUTHORS: &str = "[/rustic_authors/]";

#[inline]
async fn read_to_base64_string(path: PathBuf) -> Result<String>
{
    use base64::{engine, prelude::*};
    let image = fs::read(&path).await.map_err(|e| {
        Error::Io {
            err:  e,
            path: path.clone(),
        }
    })?;
    Ok(engine::general_purpose::STANDARD_NO_PAD.encode(image))
}

/// # Errors
///
/// Will return errors if:
///
/// - There are no source files
/// - Progress bar initialization fails
///
/// # Panics
///
/// Will panic if:
///
/// - Markdown to html conversion fails
/// - Couldn't join a thread
pub async fn build(site: Website, rebuild_all: bool) -> Result<()>
{
    use indicatif::ProgressBar;
    let site = Arc::new(site);
    let config = &site.config;
    let source_file_dir = walk_directory(&config.source);
    let source_file_count = source_file_dir.len();

    // If there's no source files we exit with an error
    if source_file_count == 0 {
        return Err(Error::MissingSourceFiles(config.source.clone()));
    }

    let pb = ProgressBar::new(source_file_count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .map_err(|_| Error::ProgressBarInitialization)?
            .progress_chars("#>-"),
    );

    // Create a task for each
    let builds = source_file_dir
        .into_iter()
        .map(|source_file| {
            let site = site.clone(); // Clone the Arc
            let pb = pb.clone();
            tokio::spawn(async move {
                Error::unwrap_gracefully(
                    site.make_html_from_md(source_file, pb.clone(), rebuild_all)
                        .await
                        .map_err(|e| {
                            pb.set_message("Failed");
                            e
                        }),
                );
            })
        })
        .collect::<Vec<_>>();

    pb.set_message("Generating ...");
    // Wait for builds to finish
    for build in builds {
        build.await.unwrap();
    }

    pb.set_message("Done");
    pb.finish();
    Ok(())
}

fn walk_directory(path: &Path) -> Vec<(PathBuf, String)>
{
    // Walk the source directory and filter the results to only include files
    // that have a markdown file extention
    #[allow(clippy::unnecessary_unwrap)]
    let contents: Vec<(PathBuf, String)> = WalkDir::new(path)
        .into_iter()
        .filter_map(|x| {
            let extention: &str = &x
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .unwrap_or(&OsString::new())
                .to_string_lossy()
                .to_lowercase();
            let x = x;
            if x.is_ok() && {
                x.as_ref().unwrap().path().is_file()
                    && (extention == "markdown" || extention == "md" || extention == "html" || extention == "htm")
            } {
                Some((x.unwrap().path().to_path_buf(), extention.to_string()))
            }
            else {
                // If x is an error we print an error, but we continue.
                if x.is_err() {
                    let e = Error::ReadSourceDir {
                        err:  x.as_ref().err().unwrap().to_string(),
                        path: PathBuf::from("UNKNOWNPATH"),
                    };
                    e.report();
                }
                None
            }
        })
        .collect();
    contents
}

/// # Errors
///
/// Will returns errors if:
///
/// - `source` path doesn't exist
/// - `dest` path doesn't exist
fn should_regenerate_file(source: &Path, dest: &Path) -> Result<bool>
{
    if dest.exists() {
        let source_path_metadata = source.metadata().map_err(|e| {
            Error::Io {
                err:  e,
                path: source.to_path_buf(),
            }
        })?;
        let dest_path_metadata = dest.metadata().map_err(|e| {
            Error::Io {
                err:  e,
                path: source.to_path_buf(),
            }
        })?;

        let source_last_modified: DateTime<Local> = source_path_metadata.modified().unwrap().into();
        let dest_last_modified: DateTime<Local> = dest_path_metadata.modified().unwrap().into();

        if source_last_modified < dest_last_modified {
            return Ok(false);
        }
    }

    Ok(true)
}

/// # Errors
///
/// Will error if:
///
/// - Syntax folder cannot be loaded from
/// - Syntax themes folder cannot be loaded from
pub fn get_syntaxes(
    config: &Config,
) -> Result<(
    syntect::parsing::SyntaxSetBuilder,
    std::collections::BTreeMap<String, highlighting::Theme>,
)>
{
    let syntax_dir = &config.syntaxes;
    let custom_syntax_themes_dir = &config.custom_syntax_themes;

    let mut syntax_set_builder = SyntaxSet::load_defaults_newlines().into_builder();
    if syntax_dir.is_dir() {
        syntax_set_builder.add_from_folder(syntax_dir, true).map_err(|e| {
            Error::LoadSyntax {
                path: syntax_dir.clone(),
                err:  e.to_string(),
            }
        })?;
    }

    let mut themes = highlighting::ThemeSet::load_defaults().themes;
    if custom_syntax_themes_dir.is_dir() {
        let custom_theme_files =
            highlighting::ThemeSet::discover_theme_paths(custom_syntax_themes_dir).map_err(|e| {
                Error::LoadSyntaxThemes {
                    err:  e.to_string(),
                    path: custom_syntax_themes_dir.clone(),
                }
            })?;


        // Get the custom themes and add them to the theme map.
        for custom_theme_file in custom_theme_files {
            let theme = highlighting::ThemeSet::get_theme(&custom_theme_file).map_err(|e| {
                Error::LoadSyntaxThemes {
                    err:  e.to_string(),
                    path: custom_syntax_themes_dir.clone(),
                }
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


pub struct Website
{
    config:         Config,
    emoji_replacer: Replacer,
    syntax_set:     SyntaxSet,
    syntax_theme:   highlighting::Theme,

    /// The text-based assets loaded into memory
    assets: Arc<DashMap<PathBuf, String>>,
}

impl Website
{
    pub fn new(
        config: Config,
        syntax_set: SyntaxSet,
        assets: Arc<DashMap<PathBuf, String>>,
        syntax_theme: highlighting::Theme,
    ) -> Self
    {
        Self {
            config,
            emoji_replacer: Replacer::new(),
            syntax_set,
            syntax_theme,
            assets,
        }
    }

    /// Parse a markdown source into html and the contained `PageInfo`
    ///
    /// # Errors
    ///
    /// Will return an error if:
    ///
    /// - Syntax highligting fails
    /// - `PageInfo` isn't parsable or is missing.
    pub fn parse_markdown(&self, source: &str, source_path: PathBuf) -> Result<(String, PageInfo)>
    {
        use pulldown_cmark::{html, Options, Parser, Tag};

        // Enable features that aren't part of the standard, but are widely
        // used.
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(source, options);

        let mut html_out = String::new();
        let mut current_language = None;
        let mut unparsed_page_info = None;
        let mut markdown_html = Vec::new();
        'next_event: for mut event in parser {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                    current_language = Some(lang.clone());

                    if lang.as_ref() == PageInfo::CODE_BLOCK_IDENTIFIER {
                        continue 'next_event;
                    }
                }
                Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                    current_language = None;

                    // Suppress templateinfo stuff
                    // Suppress templateinfo and handler stuff
                    if lang.as_ref() == PageInfo::CODE_BLOCK_IDENTIFIER {
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
                            event = Event::Html(highlighted_html.into());
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

    async fn get_stylesheet(&self, stylesheet: PathBuf) -> Result<String>
    {
        // Read the stylesheet and wrap it in html
        let stylesheet_path = stylesheet.canonicalize().unwrap_or(stylesheet);
        let stylesheet = if let Some(contents) = self.assets.get(&stylesheet_path) {
            contents.clone()
        }
        else {
            let stylesheet = format!(
                "<style>{}</style>",
                fs::read_to_string(&stylesheet_path).await.map_err(|e| {
                    Error::Io {
                        err:  e,
                        path: stylesheet_path.clone(),
                    }
                })?
            );
            self.assets.insert(stylesheet_path, stylesheet.clone());
            stylesheet
        };
        Ok(stylesheet)
    }

    async fn get_favicon(&self, favicon: PathBuf) -> Result<String>
    {
        let favicon_path = favicon.canonicalize().unwrap_or(favicon);
        let favicon_encoded = if let Some(contents) = self.assets.get(&favicon_path) {
            contents.clone()
        }
        else {
            // If the favicon isn't found then one isn't inserted.
            let encoded = if favicon_path.is_file() {
                let b64 = read_to_base64_string(favicon_path.clone()).await?;
                // Base64 encode the favicon and wrap it in the icon HTML
                format!("<link rel=\"icon\" type=\"image/x-icon\" href=\"data:image/x-icon;base64,{b64}\">",)
            }
            else {
                String::new()
            };

            self.assets.insert(favicon_path, encoded.clone());
            encoded
        };

        Ok(favicon_encoded)
    }

    /// # Errors
    ///
    /// Will return an error if
    ///
    /// - `./` cannot be canonicalized
    /// - `source_file` cannot be read into a string
    /// - The generated `dest_file` cannot be written to
    ///
    /// # Panics
    ///
    /// Will panic if:
    ///
    /// - `source_file`'s file stem cannot be extracted.
    pub async fn make_html_from_md(
        &self,
        source_file: (PathBuf, String),
        pb: indicatif::ProgressBar,
        rebuild_all: bool,
    ) -> Result<()>
    {
        let config = &self.config;
        let (source_file, source_file_extention) = source_file;
        let source_file_name = source_file.file_stem().unwrap();
        let here = PathBuf::from(".").canonicalize().map_err(|e| {
            Error::Io {
                err:  e,
                path: PathBuf::from("."),
            }
        })?;
        let source_path_stem = source_file
            .iter()
            .skip_while(|x| *x != here.file_name().unwrap())
            .skip(2)
            .collect::<PathBuf>();
        let dest_dir = config.dest.join(source_path_stem.parent().unwrap_or(&source_path_stem));

        match &*source_file_extention {
            "md" | "markdown" => (),
            "css" | "html" | "htm" => {
                let mut contents = fs::read_to_string(&source_file).await.map_err(|e| {
                    Error::Io {
                        err:  e,
                        path: source_file.clone(),
                    }
                })?;

                // Perform final actions on html
                if source_file_extention != "css" {
                    if let Some(generation) = &config.generation {
                        if generation.treat_source_as_template.unwrap_or(false) {
                            let stylesheet = self.get_stylesheet(config.default.stylesheet.clone()).await?;
                            let favicon = self.get_favicon(config.default.favicon.clone()).await?;
                            self.apply_to_template(&mut contents, None, None, &favicon, &stylesheet);
                        }
                        if let Some(process_config) = &generation.process {
                            if process_config.minify {
                                contents = post_process_html(contents);
                            }
                        }
                    }
                }

                let dest_file = dest_dir.join(source_file.file_name().unwrap());
                fs::write(&dest_file, contents).await.map_err(|e| {
                    Error::Io {
                        err:  e,
                        path: dest_file,
                    }
                })?;

                return Ok(());
            }
            _ => return Ok(()),
        }

        let dest_file = dest_dir.join(format!("{}.html", source_file_name.to_string_lossy()));

        // If the destination exists, and the source is more recent'ly modified than the
        // destination, then we skip generating this file.
        if !rebuild_all && !should_regenerate_file(&source_file, &dest_file)? {
            return Ok(());
        }

        // Parse the markdown into HTML
        let source = fs::read_to_string(&source_file).await.map_err(|e| {
            Error::Io {
                err:  e,
                path: source_file.clone(),
            }
        })?;
        let (mut html, page_info) = self.parse_markdown(&source, source_file.clone())?;
        html = match self.integrate_html_into_template(page_info, source_file, html).await {
            Ok(x) => x,
            Err(_) => return Ok(()),
        };

        // Create the parent dir in the destination path
        let dest_path_parent = dest_file.parent().unwrap_or(&dest_file);
        if !dest_path_parent.exists() {
            fs::create_dir_all(dest_path_parent).await.map_err(|e| {
                Error::Io {
                    err:  e,
                    path: dest_path_parent.to_path_buf(),
                }
            })?;
        }


        if let Some(generation) = &config.generation {
            if let Some(process_config) = &generation.process {
                if process_config.minify {
                    html = post_process_html(html);
                }
            }
        }

        // Write out the file
        fs::write(&dest_file, html).await.map_err(|e| {
            Error::Io {
                err:  e,
                path: dest_file,
            }
        })?;

        pb.inc(1);
        Ok(())
    }

    /// # Errors
    ///
    /// Will return errors if:
    ///
    /// - There is no template file
    /// - The template file couldn't be read into a string
    /// - Couldn't get a favicon/stylesheet
    pub async fn integrate_html_into_template(
        &self,
        page_info: PageInfo,
        source_file: PathBuf,
        html: String,
    ) -> Result<String>
    {
        let config = &self.config;
        let stylesheet = match page_info.style.clone() {
            Some(x) => x,
            None => config.default.stylesheet.clone(),
        };
        let template = match page_info.template.clone() {
            Some(x) => x,
            None => config.default.template.clone(),
        };
        // If the template file doesn't exist, skip this file
        if !template.is_file() {
            Error::MissingTemplate {
                source_file,
                expected_template_file: template,
            }
            .report();
            return Err(Error::IntegraionIntoTemplate);
        }

        // Get the favicon file path
        let favicon_path = page_info
            .favicon
            .clone()
            .unwrap_or(PathBuf::from(&config.default.favicon));
        let favicon_path = favicon_path.canonicalize().unwrap_or(favicon_path);
        let favicon = self.get_favicon(favicon_path).await?;
        let stylesheet = self.get_stylesheet(stylesheet).await?;

        // Add the markdown html into the template html, then write it out.
        let mut template = fs::read_to_string(&template).await.map_err(|e| {
            Error::Io {
                err:  e,
                path: template.clone(),
            }
        })?;

        self.apply_to_template(&mut template, Some(html), Some(page_info), &favicon, &stylesheet);
        Ok(template)
    }

    fn apply_to_template(
        &self,
        template: &mut String,
        html: Option<String>,
        page_info: Option<PageInfo>,
        favicon: &str,
        stylesheet: &str,
    )
    {
        if let Some(html) = html {
            *template = template.replace(TEMPLATE_NAME_BODY, html.as_ref());
        }
        if let Some(page_info) = page_info {
            use htmlescape::encode_minimal;
            let (site_name, authors) = match &page_info.meta {
                Some(meta) => (meta.site_name.as_str(), meta.authors.join(", ")),
                None => {
                    (
                        self.config
                            .default
                            .meta
                            .as_ref()
                            .map_or("", |meta| meta.site_name.as_str()),
                        self.config
                            .default
                            .meta
                            .as_ref()
                            .map_or_else(String::new, |meta| meta.authors.join(", ")),
                    )
                }
            };

            // HTML escape anything needed
            let (site_name, authors) = (encode_minimal(site_name), encode_minimal(&authors));
            let mut title = page_info.title;
            if let Some(meta) = &self.config.meta {
                if let Some(append_site_name_to_title) = &meta.append_site_name_to_title {
                    match append_site_name_to_title {
                        crate::MetaAppendSiteNameToTitle::Default(x) => {
                            if *x {
                                title.push_str(&format!(" — {site_name}"));
                            }
                        }
                        crate::MetaAppendSiteNameToTitle::Custom(s) => title.push_str(&format!("{s}{site_name}")),
                    }
                }
            }

            *template = template
                .replace(TEMPLATE_NAME_TITLE, &title)
                .replace(TEMPLATE_NAME_DESC, &page_info.description)
                .replace(TEMPLATE_NAME_SITENAME, &site_name)
                .replace(TEMPLATE_NAME_AUTHORS, &authors);
        }

        *template = template
            .replace(TEMPLATE_NAME_FAVICON, favicon)
            .replace(TEMPLATE_NAME_STYLESHEET, stylesheet);
    }
}

fn post_process_html(mut html: String) -> String
{
    // Only minify if config.generation.process.minify == true

    // Create a byte vector containing the html. We feed this into an html minifier,
    // then reconstruct a string from it.
    let mut html_b: Vec<u8> = html.as_bytes().to_vec();
    let mut cfg = minify_html::Cfg::new();
    cfg.minify_css = true;
    cfg.ensure_spec_compliant_unquoted_attribute_values = true;
    html_b = minify_html::minify(&html_b, &cfg);
    html = String::from_utf8_lossy(&html_b).to_string();
    html
}

#[cfg(test)]
mod tests
{
    use dashmap::DashMap;

    use super::*;


    #[test]
    /// Test that github-like emoji parsing works properly
    fn test_emoji_markdown_parsing()
    {
        let config = Config::default();
        let theme = highlighting::ThemeSet::load_defaults()
            .themes
            .remove(&config.syntax_theme)
            .unwrap();
        let assets: Arc<DashMap<PathBuf, String>> = Arc::new(DashMap::new());
        let site = Website::new(config, SyntaxSet::load_defaults_newlines(), assets, theme);
        let markdown = r#"```pageinfo
title = "hello world"
description = "Useless"
style = "style.css"
# The path to the HTML template to use.
template = "template.html"
```

# Hello World :smile:"#;
        let (html, _) = site.parse_markdown(markdown, PathBuf::new()).unwrap();
        assert!(html.contains('😄'));
    }

    #[test]
    /// Test that syntax-highligting works properly
    fn test_syntax_highliting_markdown_parsing()
    {
        const EXPECTED_HTML: &str =
            "<pre><code class=\"language-C\"><pre style=\"background-color:#2d2d2d;\">\n<span \
             style=\"color:#cc99cc;\">int </span><span style=\"color:#6699cc;\">main</span><span \
             style=\"color:#d3d0c8;\">()\n</span><span style=\"color:#d3d0c8;\">{\n</span><span \
             style=\"color:#d3d0c8;\">    </span><span style=\"color:#cc99cc;\">return </span><span \
             style=\"color:#f99157;\">0</span><span style=\"color:#d3d0c8;\">;\n</span><span \
             style=\"color:#d3d0c8;\">}\n</span></pre>\n</code></pre>\n";

        let config = Config::default();
        let theme = highlighting::ThemeSet::load_defaults()
            .themes
            .remove(&config.syntax_theme)
            .unwrap();
        let assets: Arc<DashMap<PathBuf, String>> = Arc::new(DashMap::new());
        let site = Website::new(config, SyntaxSet::load_defaults_newlines(), assets, theme);
        let markdown = r#"```pageinfo
title = "hello world"
description = "Useless"
style = "style.css"
# The path to the HTML template to use.
template = "template.html"
```

```C
int main()
{
    return 0;
}
```
"#;
        let (html, _) = site.parse_markdown(markdown, PathBuf::new()).unwrap();
        assert_eq!(&html, EXPECTED_HTML);
    }

    #[tokio::test]
    async fn test_file_to_base64()
    {
        const TEST_FILE_CONTENTS: &str = r#"Enim itaque aliquid excepturi. Asperiores est omnis quia sequi ipsum vel. Est assumenda accusantiumiusto.
Nam vel qui facere quia corporis. Voluptatem quo magni voluptate. Earum similique cupiditate voluptatem alias repellat
aliquid placeat qui. Aspernatur incidunt et necessitatibus dignissimos faciliset. Beatae dicta nam voluptatem possimus.
Suscipit cum excepturi aliquam ut."#;
        const TEST_FILE_B64: &str = "RW5pbSBpdGFxdWUgYWxpcXVpZCBleGNlcHR1cmkuIEFzcGVyaW9yZXMgZXN0IG9tbmlzIHF1aWEgc2VxdWkgaXBzdW0gdmVsLiBFc3QgYXNzdW1lbmRhIGFjY3VzYW50aXVtaXVzdG8uCk5hbSB2ZWwgcXVpIGZhY2VyZSBxdWlhIGNvcnBvcmlzLiBWb2x1cHRhdGVtIHF1byBtYWduaSB2b2x1cHRhdGUuIEVhcnVtIHNpbWlsaXF1ZSBjdXBpZGl0YXRlIHZvbHVwdGF0ZW0gYWxpYXMgcmVwZWxsYXQKYWxpcXVpZCBwbGFjZWF0IHF1aS4gQXNwZXJuYXR1ciBpbmNpZHVudCBldCBuZWNlc3NpdGF0aWJ1cyBkaWduaXNzaW1vcyBmYWNpbGlzZXQuIEJlYXRhZSBkaWN0YSBuYW0gdm9sdXB0YXRlbSBwb3NzaW11cy4KU3VzY2lwaXQgY3VtIGV4Y2VwdHVyaSBhbGlxdWFtIHV0Lg";
        fs::create_dir_all("/tmp/rustic-raven-tests/").await.unwrap();
        fs::write("/tmp/rustic-raven-tests/base64", TEST_FILE_CONTENTS)
            .await
            .unwrap();
        let b64 = super::read_to_base64_string(PathBuf::from("/tmp/rustic-raven-tests/base64"))
            .await
            .unwrap();
        assert_eq!(b64, TEST_FILE_B64);
    }
}
