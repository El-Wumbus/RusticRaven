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
* `base16-ocean.dark`
* `base16-eighties.dark`
* `base16-mocha.dark`
* `base16-ocean.light`
* [`InspiredGitHub`](https://github.com/sethlopezme/InspiredGitHub.tmtheme)
* `Solarized (dark)`
* `Solarized (light)`