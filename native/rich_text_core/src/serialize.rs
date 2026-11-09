//! 块/行内 → Markdown 子集串。与 parser 的 `parse`/`parse_inline` 互逆：
//! 纯文本中的语法标记一律转义，保证「字面内容」往返不变形。

use crate::ast::*;

/// 行内纯文本需转义的字符：`\` 本身与子集语法标记（`*` `_` `[` `` ` `` `$`）。
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '[' | '`' | '$') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// 行内节点 → Markdown 子集串。与 `parse_inline` 互逆。
pub fn serialize_inline(nodes: &[InlineNode]) -> String {
    nodes
        .iter()
        .map(|n| match n {
            InlineNode::Text(t) => escape_text(t),
            InlineNode::Strong(ch) => format!("**{}**", serialize_inline(ch)),
            InlineNode::Em(ch) => format!("*{}*", serialize_inline(ch)),
            InlineNode::Underline(ch) => format!("<u>{}</u>", serialize_inline(ch)),
            InlineNode::Code(c) => format!("`{}`", c),
            InlineNode::Link {
                label,
                url,
                autolink,
            } => {
                if *autolink {
                    url.clone()
                } else {
                    format!("[{}]({})", escape_text(label), url)
                }
            }
            InlineNode::Strikethrough(ch) => format!("~~{}~~", serialize_inline(ch)),
            InlineNode::Highlight(ch) => format!("=={}==", serialize_inline(ch)),
            InlineNode::Math(s) => format!("${}$", s),
        })
        .collect()
}

/// 块 → Markdown 子集串。与 `parse` 互逆（三出口护栏之 serialize）。
pub fn serialize_block(block: &RichBlock) -> String {
    match block {
        RichBlock::Heading { level, inline } => {
            format!("{} {}", "#".repeat(*level as usize), serialize_inline(inline))
        }
        RichBlock::Paragraph(inline) => serialize_inline(inline),
        RichBlock::Quote(children) => children
            .iter()
            .map(serialize_block)
            .map(|md| {
                md.split('\n')
                    .map(|l| format!("> {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n>\n"),
        RichBlock::List { ordered, items } => items
            .iter()
            .enumerate()
            .map(|(k, it)| {
                let lines = &it.lines;
                let first = serialize_inline(&lines[0]);
                let head = match it.done {
                    Some(done) => format!("- [{}] {first}", if done { "x" } else { " " }),
                    None => {
                        if *ordered {
                            format!("{}. {first}", k + 1)
                        } else {
                            format!("- {first}")
                        }
                    }
                };
                let cont = lines
                    .iter()
                    .skip(1)
                    .map(|l| format!("  {}", serialize_inline(l)))
                    .collect::<Vec<_>>()
                    .join("\n");
                if cont.is_empty() {
                    head
                } else {
                    format!("{head}\n{cont}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        RichBlock::CodeBlock { code, language } => {
            format!("```{}", language.clone().unwrap_or_default())
                + "\n"
                + code
                + "\n```"
        }
        RichBlock::Divider => "---".to_string(),
        RichBlock::Image { url, alt } => format!("![{}]({})", escape_text(alt), url),
        RichBlock::Audio { url, label } => format!("[{}]({})", escape_text(label), url),
        RichBlock::Video { url, label } => format!("[{}]({})", escape_text(label), url),
        RichBlock::MathBlock(s) => "$$".to_string() + s + "$$",
        RichBlock::Table {
            header,
            rows,
            align,
        } => serialize_table(header, rows, align),
    }
}

/// 表格 → Markdown 子集串（GFM 管线语法；无尾随换行，交由 [serialize_blocks] 块间空行）。
fn serialize_table(
    header: &[String],
    rows: &[Vec<String>],
    align: &Option<Vec<TableAlign>>,
) -> String {
    let row = |cells: &[String]| format!("| {} |", cells.join(" | "));
    let a = align
        .clone()
        .unwrap_or_else(|| vec![TableAlign::Left; header.len()]);
    let sep = format!(
        "| {} |",
        a.iter()
            .map(|al| match al {
                TableAlign::Center => ":---:".to_string(),
                TableAlign::Right => "---:".to_string(),
                TableAlign::Left => ":---".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" | ")
    );
    let mut buf = String::new();
    buf.push_str(&row(header));
    buf.push('\n');
    buf.push_str(&sep);
    buf.push('\n');
    for r in rows {
        buf.push_str(&row(r));
        buf.push('\n');
    }
    buf.trim_end().to_string()
}

/// 块列表 → Markdown 子集串（块间空行分隔；编辑器回写唯一出口）。
pub fn serialize_blocks(blocks: &[RichBlock]) -> String {
    blocks
        .iter()
        .map(serialize_block)
        .collect::<Vec<_>>()
        .join("\n\n")
}
