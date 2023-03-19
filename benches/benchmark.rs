use std::{path::PathBuf, sync::Arc, time::Duration};

// This is a struct that tells Criterion.rs to use the "futures" crate's current-thread executor
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dashmap::DashMap;
use rustic_raven::{build::Website, defaults, Config};
use syntect::{highlighting, parsing::SyntaxSet};

fn benchmark_parse_markdown(c: &mut Criterion)
{
    let config = Config::default();
    let theme = highlighting::ThemeSet::load_defaults()
        .themes
        .remove(&config.syntax_theme)
        .unwrap();
    let assets: Arc<DashMap<PathBuf, String>> = Arc::new(DashMap::new());
    let site = Website::new(config, SyntaxSet::load_defaults_newlines(), assets, theme);
    let markdown = DEFAULT_MD_BENCHMARK_SRC;
    let mut group = c.benchmark_group("throughput");
    group.throughput(criterion::Throughput::Bytes(markdown.bytes().len() as u64));
    group
        .sample_size(10_000)
        .measurement_time(Duration::from_secs(15))
        .significance_level(0.08);
    group.bench_function("html_from_markdown DEFAULT_MD_BENCHMARK_SRC", |b| {
        b.iter(|| {
            site.parse_markdown(black_box(markdown), PathBuf::new()).unwrap();
        });
    });
    group.finish();
}

fn benchmark_integrate_html_into_template(c: &mut Criterion)
{
    let config = Config::default();
    let theme = highlighting::ThemeSet::load_defaults()
        .themes
        .remove(&config.syntax_theme)
        .unwrap();
    let assets: Arc<DashMap<PathBuf, String>> = Arc::new(DashMap::new());
    let site = Website::new(config.clone(), SyntaxSet::load_defaults_newlines(), assets, theme);
    let markdown = DEFAULT_MD_BENCHMARK_SRC;
    let (html, page_info) = site.parse_markdown(black_box(markdown), PathBuf::new()).unwrap();
    let stylesheet = match page_info.style.clone() {
        Some(x) => x,
        None => config.default.stylesheet.clone(),
    };
    let template = match page_info.template.clone() {
        Some(x) => x,
        None => config.default.template,
    };
    std::fs::write(stylesheet, defaults::DEFAULT_CSS_STYLESHEET_SRC).unwrap();
    std::fs::write(template, defaults::DEFAULT_HTML_TEMPLATE_SRC).unwrap();

    let exe = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("throughput");
    group.throughput(criterion::Throughput::Bytes(html.bytes().len() as u64));
    group
        .sample_size(10_000)
        .measurement_time(Duration::from_secs(10))
        .noise_threshold(0.13);
    group.bench_function("benchmark_integrate_html_into_template DEFAULT_MD_BENCHMARK_SRC", |b| {
        b.to_async(&exe)
            .iter(|| site.integrate_html_into_template(page_info.clone(), PathBuf::new(), html.clone()));
    });
    group.finish();
}

criterion_group!(
    benches,
    benchmark_parse_markdown,
    benchmark_integrate_html_into_template
);
criterion_main!(benches);


const DEFAULT_MD_BENCHMARK_SRC: &str = r#"# RusticRaven

## Installation

### Releases

You can pick the latest release archive from the [GitHub Releases](https://github.com/El-Wumbus/RusticRaven/releases/latest).

#### Snap
##### Building the snap yourself

In some cases, you may want to build the snap yourself.
To do this, you'll need to have `snapd` installed and have installed the following snaps:

- `lxd`
  - [This may require a little configuration](https://ubuntu.com/server/docs/containers-lxd)
- `snapcraft`
- `multipass`

```bash
git clone https://github.com/El-Wumbus/RusticRaven
cd RusticRaven

snapcraft # Build he snap
sudo snap install --dangerous rustic-raven_*.snap # Install the snap
```
## Usage

The usage information of the project can be obtained with the `--help` option.

```
RusticRaven

USAGE:
    raven <SUBCOMMAND>
...
```

To get the usage information of a subcommand, do something like the following: `raven help <subcommand>` or `raven <subcommand> --help`.

### Setting up a project

To create a new project, use the `new` or `init` subcommands.

```sh
$ raven new foo --dest docs
Created: "raven.toml"
Created: "src"
Created: "docs"
Created: "syntaxes"
Created: "syntax-themes"
Created: "template.html"
Created: "style.css"
Created: "src/index.md"
```

`foo` now contains all the above listed files. This is the default project and is fully buildable. To do so, use `build`.
You can build by `cd`ing into the new directory or by passing in the new directory (`raven build foo`).

```sh
# foo/
$ raven build
[00:00:00] [########################################] 1/1 (100%) Done                                                                                             
```

Now, in the `foo/docs` directory is the `index.html` file. Preview it in a web browser. By default the html is minified.

### Configuration :page_facing_up:

To make a new project with the defualt configuration run the `init` subcommand.
The default configuration looks similar to below

| Name                   | Description                                               |
| ---------------------- | --------------------------------------------------------- |
| `source`               | Where Markdown source files are stored                    |
| `dest`                 | Where generated HTML files are stored                     |
| `syntaxes`             | Where additional syntax highliting files are stored       |
| `syntax_theme`         | The syntax highlighting theme to use                      |
| `custom_syntax_themes` | Where custom syntax highlighting themes are stored        |
| `default_favicon`      | The defualt favxicon used for files that don't supply one |
| `process_html`         | If generated HTML should be processed (minimized, etc.)   |

The defualt syntax themes are as follows:
- `base16-ocean.dark`
- `base16-eighties.dark`
- `base16-mocha.dark`
- `base16-ocean.light`
- [`InspiredGitHub`](https://github.com/sethlopezme/InspiredGitHub.tmtheme)
- `Solarized (dark)`
- `Solarized (light)`

To add a custom syntax theme, add a sublime-syntax file (e.g. `TOML.sublime-syntax`) into the `syntaxes` directory. This file describes what to use in the markdown (what comes after the `` ``` ``).

#### Page Info

In each markdown file a code block with the language specifier `pageinfo` is required, it should look similar to below. It is parsed as TOML and is **not** included in the final HTML document.

The first two items here are self-explainatory. `style` is the stylesheet to be embeded into the HTML document.
It's path is relative to the `raven.toml` at the root of the project, the same thing is true in regard to the `template` and `favicon` keys.
`template` is the HTML template to embed the generated HTML into, each document can use whichever template that is available.
`favicon` is optional: if it's omitted, or the file isn't found, then the generated HTML doesn't have a favicon.
The favicon is encoded in base64 and stored using a data url in the generated HTML.
The favicon is not copied to the configured destination directory.

### Considerations

#### File handling

- Markdown files (`.md` or `.markdown`) in the configured source directory will be parsed and generated into HTML files in the configured destination directory.
- HTML files (`.html` or `.htm`) in the configured source directory will be copied to the configured destination deirectory (after, if enabled, processing).
- Everything else in the configured source directory gets ignored.

```pageinfo
title = "Hello, World"
description = "Greet the world"
style = "/tmp/rustic-raven-tests/style.css"
template = "/tmp/rustic-raven-tests/template.html"
```
"#;
