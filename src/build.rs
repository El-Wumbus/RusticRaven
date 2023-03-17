use std::sync::Arc;

use chrono::{DateTime, Local};
use dashmap::DashMap;

use crate::*;

pub struct Website
{
    config:         Config,
    emoji_replacer: Replacer,
    syntax_set:     SyntaxSet,
    syntax_theme:   highlighting::Theme,
}

impl Website
{
    pub fn new(config: Config, syntax_set: SyntaxSet, syntax_theme: highlighting::Theme) -> Self
    {
        Self {
            config,
            emoji_replacer: Replacer::new(),
            syntax_set,
            syntax_theme,
        }
    }

    async fn read_to_base64_string(&self, path: PathBuf) -> Result<String>
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

    fn parse_markdown(&self, source: String, source_path: PathBuf) -> Result<(String, PageInfo)>
    {
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

    pub async fn make_html_from_md(
        &self,
        source_file: PathBuf,
        open_assets: Arc<DashMap<PathBuf, String>>,
        pb: indicatif::ProgressBar,
        rebuild_all: bool,
    ) -> Result<()>
    {
        let assets = open_assets.clone();
        let config = &self.config;
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
        let dest_path = config
            .dest
            .join(source_path_stem.parent().unwrap_or(&source_path_stem))
            .join(format!("{}.html", source_file_name.to_string_lossy()));

        // If the destination exists, and the source is more recently modified than the
        // destination, then we skip generating this file.

        if !rebuild_all && !should_regenerate_file(&source_file, &dest_path)? {
            return Ok(());
        }

        // Parse the markdown into html
        let source = fs::read_to_string(&source_file).await.map_err(|e| {
            Error::Io {
                err:  e,
                path: source_file.clone(),
            }
        })?;

        let (html, page_info) = Error::unwrap_gracefully(self.parse_markdown(source, source_file.clone()));
        let template = page_info.template;
        let stylesheet = page_info.style;

        // If the template file doesn't exist, skip this file
        if !template.is_file() {
            Error::MissingTemplate {
                source_file,
                expected_template_file: template,
            }
            .report();
            return Ok(());
        }

        // Get the favicon file path
        let favicon_path = page_info.favicon.unwrap_or(PathBuf::from(&config.default_favicon));

        // If the favicon file doesn't exist, skip this file.
        if !favicon_path.is_file() {
            Error::MissingFavicon {
                source_file,
                expected_favicon_file: favicon_path,
            }
            .report();
            return Ok(());
        }
        let favicon_path = favicon_path.canonicalize().unwrap_or(favicon_path);
        let favicon_encoded = if let Some(contents) = assets.get(&favicon_path) {
            contents.clone()
        }
        else {
            // Base64 encode the favicon and wrap it in the icon HTML
            let encoded = format!(
                "<link rel=\"icon\" type=\"image/x-icon\" href=\"data:image/x-icon;base64,{}\">",
                self.read_to_base64_string(favicon_path.clone()).await?
            );

            assets.insert(favicon_path, encoded.clone());
            encoded
        };


        // Read the stylesheet and wrap it in html
        let stylesheet_path = stylesheet.canonicalize().unwrap_or(stylesheet);
        let stylesheet = if let Some(contents) = open_assets.get(&stylesheet_path) {
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

            assets.insert(stylesheet_path, stylesheet.clone());
            stylesheet
        };


        // Add the markdown html into the template html, then write it out.
        let html = Error::unwrap_gracefully(fs::read_to_string(&template).await.map_err(|e| {
            Error::Io {
                err:  e,
                path: template.clone(),
            }
        }))
        .replace(TEMPLATE_NAME_BODY, &html)
        .replace(TEMPLATE_NAME_TITLE, &page_info.title)
        .replace(TEMPLATE_NAME_DESC, &page_info.description)
        .replace(TEMPLATE_NAME_FAVICON, &favicon_encoded)
        .replace(TEMPLATE_NAME_STYLESHEET, &stylesheet);

        // Create the parent dir in the destination path
        let dest_path_parent = dest_path.parent().unwrap_or(&dest_path);
        if !dest_path_parent.exists() {
            fs::create_dir_all(dest_path_parent).await.map_err(|e| {
                Error::Io {
                    err:  e,
                    path: dest_path_parent.to_path_buf(),
                }
            })?;
        }

        // Perform final actions on html
        let html = post_process_html(html)?;

        // Write out the file
        Error::unwrap_gracefully(fs::write(&dest_path, html).await.map_err(|e| {
            Error::Io {
                err:  e,
                path: dest_path,
            }
        }));

        pb.inc(1);
        Ok(())
    }
}

pub async fn build(site: Website, rebuild_all: bool) -> Result<()>
{
    use indicatif::{ProgressBar};
    let site = Arc::new(site);
    let config = &site.config;
    let source_file_dir = walk_directory(&config.source);
    let source_file_count = source_file_dir.len();

    // If there's no source files we exit with an error
    if source_file_count == 0 {
        return Err(Error::MissingSourceFiles(config.source.clone()));
    }

    // The assets we've already loaded.
    // We use an Arc<DashMap> over an Arc<Mutex<Hashmap>> for finer-grained locking.
    // The changes are still syncronized.
    let open_assets: Arc<DashMap<PathBuf, String>> = Arc::new(DashMap::new());
    let pb = ProgressBar::new(source_file_count as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Create a task for each
    let builds = source_file_dir
        .into_iter()
        .map(|source_file| {
            let open_assets = open_assets.clone(); // Clone the Arc
            let site = site.clone(); // Clone the Arc
            let pb = pb.clone();
            tokio::spawn(async move {
                Error::unwrap_gracefully(
                    site.make_html_from_md(source_file, open_assets, pb.clone(), rebuild_all)
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

fn walk_directory(path: &Path) -> Vec<PathBuf>
{
    // Walk the source directory and filter the results to only include files
    // that have a markdown file extention
    #[allow(clippy::unnecessary_unwrap)]
    let contents: Vec<PathBuf> = WalkDir::new(path)
        .into_iter()
        .filter_map(|x| {
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
                Some(x.unwrap().path().to_path_buf())
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
            let e = Error::LoadSyntax {
                path: syntax_dir.to_path_buf(),
                err:  e.to_string(),
            };

            // Report the error and exit if this fails
            e.report_and_exit()
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

fn post_process_html(html: String) -> Result<String>
{
    // Create a byte vector containing the html. We feed this into an html minifier,
    // then reconstruct a string from it.
    let mut html: Vec<u8> = html.as_bytes().to_vec();
    let mut cfg = minify_html::Cfg::new();
    cfg.minify_css = true;
    cfg.ensure_spec_compliant_unquoted_attribute_values = true;
    html = minify_html::minify(&html, &cfg);
    let html = String::from_utf8_lossy(&html);
    Ok(html.to_string())
}

#[cfg(test)]
mod tests
{
    use crate::*;
    const SYNTAX_HIGHLIGHT_THEME: &str = "base16-eighties.dark";

    #[test]
    /// Test that github-like emoji parsing works properly
    fn test_emoji_markdown_parsing()
    {
        let theme = highlighting::ThemeSet::load_defaults()
            .themes
            .remove(SYNTAX_HIGHLIGHT_THEME)
            .unwrap();
        let site = Website::new(Config::default(), SyntaxSet::load_defaults_newlines(), theme);
        let markdown = r#"```pageinfo
title = "hello world"
description = "Useless"
style = "style.css"
# The path to the HTML template to use.
template = "template.html"
```

# Hello Word :smile:"#;
        let (html, _) = site.parse_markdown(markdown.to_string(), PathBuf::new()).unwrap();
        assert_eq!(&html, "<h1>Hello Word 😄</h1>\n")
    }

    #[test]
    /// Test that syntax-highligting works properly
    fn test_syntax_highliting_markdown_parsing()
    {
        let theme = highlighting::ThemeSet::load_defaults()
            .themes
            .remove(SYNTAX_HIGHLIGHT_THEME)
            .unwrap();
        let site = Website::new(Config::default(), SyntaxSet::load_defaults_newlines(), theme);
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

        const EXPECTED_HTML: &str =
            "<pre><code class=\"language-C\"><pre style=\"background-color:#2d2d2d;\">\n<span \
             style=\"color:#cc99cc;\">int </span><span style=\"color:#6699cc;\">main</span><span \
             style=\"color:#d3d0c8;\">()\n</span><span style=\"color:#d3d0c8;\">{\n</span><span \
             style=\"color:#d3d0c8;\">    </span><span style=\"color:#cc99cc;\">return </span><span \
             style=\"color:#f99157;\">0</span><span style=\"color:#d3d0c8;\">;\n</span><span \
             style=\"color:#d3d0c8;\">}\n</span></pre>\n</code></pre>\n";
        let (html, _) = site.parse_markdown(markdown.to_string(), PathBuf::new()).unwrap();
        assert_eq!(&html, EXPECTED_HTML);
    }
}
