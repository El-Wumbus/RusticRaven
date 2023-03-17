pub const DEFAULT_HTML_TEMPLATE_SRC: &str = r#"<!DOCTYPE html><html lang="en"><meta charset="UTF-8"><meta content="IE=edge" http-equiv="X-UA-Compatible"><meta content="width=device-width,initial-scale=1" name="viewport"><meta content="[/rustic_title/]" property="og:title"><meta content="[/rustic_description/]" property="og:description">[/rustic_favicon/]<title>[/rustic_title/]</title>[/rustic_stylesheet/] [/rustic_body/]"#;
pub const DEFAULT_CSS_STYLESHEET_SRC: &str = r#":root{background-color:#282828;color:#e7d7ad}pre{border-width:0;padding:2px;border-radius:5px;scrollbar-width:5px}pre code{border-width:0;border-radius:5px;font-size:1em;padding:2px}"#;
pub const DEFAULT_MD_STARTER_SRC: &str = r#"# Hello, World! :wave: :world_map:

```C
#include <stdio.h>

int main()
{
    printf("Hello, World!");
    return 0;
}
```

| Name  | Greeeting     |
| ----- | ------------- |
| World | Hello, World! |
| James | Hello, James! |


```pageinfo
title = "Hello, World"
description = "Greet the world"
style = "style.css"
template = "template.html"
```"#;
