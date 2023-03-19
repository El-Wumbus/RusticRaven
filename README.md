# RusticRaven

## Installation

[![Get it from the Snap Store](https://snapcraft.io/static/images/badges/en/snap-store-black.svg)](https://snapcraft.io/rustic-raven)

### Releases

You can pick the latest release archive from the [GitHub Releases](https://github.com/El-Wumbus/RusticRaven/releases/latest).

#### Snap

To download the latest [snap release](https://snapcraft.io/rustic-raven/) you can use the following commands:

```sh
sudo snap install rustic-raven --edge
```
In this case the executable will be named differently than with other installation options (`rustic-raven.raven`).

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

snapcraft # Build the snap
sudo snap install --dangerous rustic-raven_*.snap # Install the snap
```

### Compiling

#### PKGBUILD (Arch Linux and its derivatives)

```bash
curl -LO https://github.com/El-Wumbus/RusticRaven/raw/master/PKGBUILD
makepkg -si
```

#### Cargo (Everyone)

```bash
git clone https://github.com/El-Wumbus/RusticRaven
cd RusticRaven
cargo build --release
sudo install -dvm755 target/release/raven /usr/local/bin/raven
```

## Usage

The usage information of the project can be obtained with the `--help` option.

```
RusticRaven

USAGE:
    raven <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    build    Build static HTML from an existing project
    clean    Clean the dest dir of generated files and directories
    help     Prints this message or the help of the given subcommand(s)
    init     Initialize a new project
    new      Create a new directory and initalize it
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

A configuration may look similar to below:

```toml
source = "src"
dest = "dest"
syntaxes = "syntaxes"
syntax_theme = "base16-eighties.dark"
custom_syntax_themes = "syntax-themes"

[default]
favicon = "favicon.ico"
stylesheet = "style.css"
template = "template.html"

[default.meta]
site_name = "Rustic Raven"
authors = []

[meta]
append_site_name_to_title = true

[generation]
process = { minify = true }
treat_source_as_template = true
```

| Field                                 | Type                          | Description                                                               | Required? |
| ------------------------------------- | ----------------------------- | ------------------------------------------------------------------------- | --------- |
| `source`                              | Path (String)                 | Where Markdown source files are stored                                    | Yes       |
| `dest`                                | Path (String)                 | Where generated HTML files are stored                                     | Yes       |
| `syntaxes`                            | Path (String)                 | Where additional syntax highliting files are stored                       | Yes       |
| `custom_syntax_themes`                | Path (String)                 | Where custom syntax highlighting themes are stored                        | Yes       |
| `syntax_theme`                        | String                        | The syntax highlighting theme to use                                      | Yes       |
| `default`                             | Table                         | Default values that can be overridden in indviviual files                 | Yes       |
| `default.favicon`                     | Path (String)                 | The defualt favicon used for files that don't supply one                  | Yes       |
| `default.stylesheet`                  | Path (String)                 | The default CSS stylesheet used for files that don't specify one          | Yes       |
| `default.template`                    | Path (String)                 | The default HTML template used for files that don't specify one           | Yes       |
| `default.meta`                        | Table                         | The default metadata for a page (if the page doesn't supply it)           | No        |
| `default.meta.site_name`              | String                        | The default name of the website                                           | Yes       |
| `default.meta.authors`                | Array\[String]                | The default author(s) of a page                                           | Yes       |
| `meta`                                | Table                         | Settings releated to Metadata insertion into HTML                         | No        |
| `meta.append_site_name_to_title`      | [Boolean *OR* String][masntt] | Append the site name to a page's given title                              | No        |
| `generation`                          | Table                         | Settings related to HTML generation                                       | No        |
| `generation.process`                  | Table                         | Settings related to proccessing generated HTML                            | No        |
| `generation.process.minify`           | Boolean                       | Wether generated HTML should be processed (minimized, etc.)               | Yes       |
| `generation.treat_source_as_template` | Boolean                       | Wether to allow usage of templating in HTML files in the source directory | No        |

The defualt syntax themes are as follows:
- `base16-ocean.dark`
- `base16-eighties.dark`
- `base16-mocha.dark`
- `base16-ocean.light`
- [`InspiredGitHub`](https://github.com/sethlopezme/InspiredGitHub.tmtheme)
- `Solarized (dark)`
- `Solarized (light)`

To add a custom syntax theme, add a sublime-syntax file (e.g. `TOML.sublime-syntax`) into the `syntaxes` directory. This file describes what to use in the code block language names(what comes after the `` ``` ``).

##### `meta.append_site_name_to_title`

The possible values per type:

- `Boolean`
  - `true`: Append and use the default separation
  - `false`: Do not append
- `String`: Append and use the supplied separation

Example (Assume: PageTitle = "Hello", SiteName = "CoolSite"): 

```toml
[meta]
append_site_name_to_title = true
```

Would result in a page title: `Hello — Coolsite`

Example2 (Assume: PageTitle = "Hello", SiteName = "CoolSite"):

```toml
[meta]
append_site_name_to_title = " | "
```

Would result in a page title: `Hello | Coolsite`

#### Page Info

In each markdown file a code block with the language specifier `pageinfo` is required, it should look similar to below. It is parsed as TOML and is **not** included in the final HTML document.

````markdown
```pageinfo
title = "Hello, World"
description = "Greet the world"

# optional
style = "style.css"
template = "template.html"
favicon = favicon.ico

[meta]
site_name = "Rustic Raven"
authors = []
```
````

| Field            | Type           | Description                                           | Required? |
| ---------------- | -------------- | ----------------------------------------------------- | --------- |
| `title`          | String         | The title of the page                                 | Yes       |
| `description`    | String         | The description of the page                           | Yes       |
| `style`          | Path (String)  | The CSS stylesheet to use, this overrides the default | No        |
| `template`       | Path (String)  | The HTML template to use, this overrides the default  | No        |
| `favicon`        | Path (String)  | The favicon image to use for the page                 | No        |
| `meta`           | Table          | The metadata for the page                             | No        |
| `meta.site_name` | String         | The name of the website                               | Yes       |
| `meta.authors`   | Array\[String] | The author(s) of the page                             | Yes       |

The favicon and stylesheet are embeded into the HTML document.
The favicon is encoded in base64 and stored using a data url in the generated HTML, it is not copied to the destination directory.
The paths for all the fields are relative to the `raven.toml` at the root of the project.

[masntt]: #metaappend_site_name_to_title

### Considerations

#### File handling

- Markdown files (`.md` or `.markdown`) in the configured source directory will be parsed and generated into HTML files in the configured destination directory.
- HTML files (`.html` or `.htm`) in the configured source directory will be copied to the configured destination deirectory (after, if enabled, processing).
- CSS files (`.css`) in the configured source directory will be copied to the configured destination directory.
- Everything else in the configured source directory gets ignored.
