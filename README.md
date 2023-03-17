# RusticRaven

## Usage

The usage information of the project can be obtained with the `--help` option.

```
RusticRaven
A static html generator

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

```
Create a new directory and initalize it

USAGE:
    raven new [OPTIONS] <name>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --dest <dest>                      The name of the output directory (Where the generated HTML goes)
    -s, --source <source>                  The name of the source directory
    -t, --syntax_themes <syntax-themes>    The name of the custom syntax themes directory
    -y, --syntaxes <syntaxes>              The name of the custom syntax directory

ARGS:
    <name>    The name of the new project
```

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

```
# foo/
$ raven build
[00:00:00] [########################################] 1/1 (100%) Done                                                                                             
```

Now, in the `foo/docs` directory is the `index.html` file. Preview it in a web browser. By default the html is minified.

### Configuration :page_facing_up:

To make a new project with the defualt configuration run the `init` subcommand.
The default configuration looks similar to below:

```toml
# raven.toml

source = "src"
dest = "dest"
syntaxes = "syntaxes"
syntax_theme = "base16-eighties.dark"
custom_syntax_themes = "syntax-themes"
default_favicon = "favicon.png"
```

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

````markdown
```pageinfo
title = "Hello, World"
description = "Greet the world"
style = "style.css"
template = "template.html"

# optional
favicon = favicon.ico
```
````

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
