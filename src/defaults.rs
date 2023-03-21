pub const DEFAULT_HTML_TEMPLATE_SRC: &str = r#"<!DOCTYPE html><html lang="en"><meta charset="UTF-8"><meta content="IE=edge" http-equiv="X-UA-Compatible"><meta content="width=device-width,initial-scale=1" name="viewport"><meta content="[/rustic_title/]" property="og:title"><meta content="[/rustic_description/]" property="og:description">[/rustic_favicon/]<title>[/rustic_title/]</title>[/rustic_stylesheet/] [/rustic_body/]"#;
pub const DEFAULT_CSS_STYLESHEET_SRC: &str = r#"body{background:#282828;font-size:1em;color:#e7d7ad}pre,pre pre{border:0;padding:0;border-radius:5px;margin:0}pre code{border:0;border-radius:5px;font-size:1em;padding:0;margin:0}pre code div{scrollbar-width:5px;background:#2d2d2d;border:0;border-radius:.5rem;font-size:1em;padding-top:1rem;padding-bottom:.5rem;padding-inline:1.25rem;overflow-x:scroll;margin:0;margin-bottom:1rem}table{border:1px solid #32302f;padding:0}td,th{border-top:1px solid #32302f;padding-inline:.5rem;text-align:left}th{background:#98971a;border-bottom:1px}tr:nth-child(even){background:#242424}"#;
pub const DEFAULT_MD_STARTER_SRC: &str = r#"# Hello, World! :wave: :world_map:

```C
#include <stdio.h>

int main() // Our entry point
{
    printf("Hello, World!");
    return 0;
}
```

An explaination of the code above:

1. We `#include <stdio.h>` to say we want to use the standard Input/Output library.
2. We make a `main` function and have it return an integer.
3. We print our greeting.
4. We return `0` (meaning success).


| ***Name*** | ***Greeeting*** | ***Website***                              |
| -------- | ------------- | ---------------------------------------- |
| World    | Hello, World! | [*example.com*](https://www.example.com) |
| James    | Hello, James! | [*example.org*](https://www.example.org) |


```pageinfo
title = "Hello, World"
description = "Greet the world"
style = "style.css"
template = "template.html"
```"#;
