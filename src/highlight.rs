use anyhow::Context;
use tree_sitter_md::{MarkdownParser, MarkdownTree};

// https://en.wikipedia.org/wiki/ANSI_escape_code
static STYLE_BOLD: &str = "\x1b[1m";
static STYLE_DEFAULT_BG: &str = "\x1b[49m";
static STYLE_DEFAULT_FG: &str = "\x1b[39m";
static STYLE_GRAY_BG: &str = "\x1b[100m";
static STYLE_REGULAR: &str = "\x1b[22m";
static STYLE_YELLOW_FG: &str = "\x1b[33m";

enum TagKind {
    CodeBegin,
    CodeEnd,
    HeadingBegin,
    HeadingEnd,
    StrongBegin,
    StrongEnd,
}

struct Tag {
    kind: TagKind,
    position: usize,
}

impl Tag {
    fn new(kind: TagKind, position: usize) -> Self {
        Self { kind, position }
    }
}

pub fn highlight_markdown(source: &str) -> anyhow::Result<String> {
    let mut parser = MarkdownParser::default();
    let tree = parser
        .parse(source.as_bytes(), None)
        .context("Could not parse Markdown")?;

    let mut highlighted = String::new();
    let mut position: usize = 0;
    for tag in find_tags(&tree) {
        highlighted.push_str(&source[position..tag.position]);
        let style = match tag.kind {
            TagKind::CodeBegin => STYLE_GRAY_BG,
            TagKind::CodeEnd => STYLE_DEFAULT_BG,
            TagKind::HeadingBegin => STYLE_YELLOW_FG,
            TagKind::HeadingEnd => STYLE_DEFAULT_FG,
            TagKind::StrongBegin => STYLE_BOLD,
            TagKind::StrongEnd => STYLE_REGULAR,
        };
        highlighted.push_str(style);
        position = tag.position;
    }
    highlighted.push_str(&source[position..]);

    Ok(highlighted)
}

fn find_tags(tree: &MarkdownTree) -> impl Iterator<Item = Tag> {
    let mut visited = false;
    let mut cursor = tree.walk();
    let mut tags: Vec<Tag> = Vec::new();
    loop {
        let node = cursor.node();
        if !visited {
            match node.kind() {
                "atx_heading" => {
                    tags.push(Tag::new(TagKind::HeadingBegin, node.start_byte()));
                    tags.push(Tag::new(TagKind::HeadingEnd, node.end_byte()));
                }
                "code_span" | "fenced_code_block" => {
                    tags.push(Tag::new(TagKind::CodeBegin, node.start_byte()));
                    tags.push(Tag::new(TagKind::CodeEnd, node.end_byte()));
                }
                "strong_emphasis" => {
                    tags.push(Tag::new(TagKind::StrongBegin, node.start_byte()));
                    tags.push(Tag::new(TagKind::StrongEnd, node.end_byte()));
                }
                _ => {
                    // println!("{}", node.kind());
                }
            }
        }
        if !visited && cursor.goto_first_child() {
            continue;
        }
        if cursor.goto_next_sibling() {
            visited = false;
            continue;
        }
        if cursor.goto_parent() {
            visited = true;
            continue;
        }
        break;
    }
    tags.sort_by_key(|tag| tag.position);
    tags.into_iter()
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn markdown() -> Result<()> {
        let source = r#"# The Title

Some **bold text** and `inline code`. Now a list:

* **First** point: list items can also contain bold text
* **Second** point

And finally a code block:

```js
const x = 42;
// "**" is the exponentiation operator
const y = x ** 2 ** 0.5;
```
"#;
        let highlighted = highlight_markdown(&source)?;
        assert_eq!(
            highlighted,
            "\x1b[33m# The Title\n\x1b[39m\
\n\
Some \x1b[1m**bold text**\x1b[22m and \x1b[100m`inline code`\x1b[49m. Now a list:\n\
\n\
* \x1b[1m**First**\x1b[22m point: list items can also contain bold text\n\
* \x1b[1m**Second**\x1b[22m point\n\
\n\
And finally a code block:\n\
\n\
\x1b[100m```js\n\
const x = 42;\n\
// \"**\" is the exponentiation operator\n\
const y = x ** 2 ** 0.5;\n\
```\n\x1b[49m\
"
        );
        Ok(())
    }
}
