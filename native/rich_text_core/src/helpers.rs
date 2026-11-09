//! 下游 helper：纯文本投影、AI 写入归一、媒体资产保全、待办扫描。
//! 与 Dart `markdown_subset.dart` 下游函数 1:1 镜像。

use crate::ast::*;
use crate::parser::{parse, parse_inline};
use crate::regexes::*;
use std::collections::{HashMap, HashSet};

/// 行内节点 → 纯文本（待办回调取文本、复制、检索预览用）。只取「人读到的字」，不含标记符号。
pub fn inline_to_plain(nodes: &[InlineNode]) -> String {
    nodes
        .iter()
        .map(|n| match n {
            InlineNode::Text(t) => t.clone(),
            InlineNode::Strong(ch) => inline_to_plain(ch),
            InlineNode::Em(ch) => inline_to_plain(ch),
            InlineNode::Underline(ch) => inline_to_plain(ch),
            InlineNode::Code(c) => c.clone(),
            InlineNode::Link { label, .. } => label.clone(),
            InlineNode::Strikethrough(ch) => inline_to_plain(ch),
            InlineNode::Highlight(ch) => inline_to_plain(ch),
            InlineNode::Math(s) => s.clone(),
        })
        .collect()
}

/// 块 → 纯文本（预览 / 检索用）。
pub fn block_to_plain(block: &RichBlock) -> String {
    match block {
        RichBlock::Heading { inline, .. } => inline_to_plain(inline),
        RichBlock::Paragraph(inline) => inline_to_plain(inline),
        RichBlock::Quote(children) => children
            .iter()
            .map(block_to_plain)
            .collect::<Vec<_>>()
            .join("\n"),
        RichBlock::List { items, .. } => items
            .iter()
            .map(|it| {
                it.lines
                    .iter()
                    .map(|l| inline_to_plain(l))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join("\n"),
        RichBlock::CodeBlock { code, .. } => code.clone(),
        RichBlock::Divider => String::new(),
        RichBlock::Image { alt, .. } => {
            if alt.is_empty() {
                "[图片]".to_string()
            } else {
                format!("[图片: {alt}]")
            }
        }
        RichBlock::Audio { label, .. } => {
            if label.is_empty() {
                "[音频]".to_string()
            } else {
                format!("[音频: {label}]")
            }
        }
        RichBlock::Video { label, .. } => {
            if label.is_empty() {
                "[视频]".to_string()
            } else {
                format!("[视频: {label}]")
            }
        }
        RichBlock::MathBlock(s) => s.clone(),
        RichBlock::Table { header, rows, .. } => {
            let mut v = header.clone();
            for r in rows {
                v.extend(r.clone());
            }
            v.join(" ")
        }
    }
}

/// Markdown 子集 → 纯文本（整篇剥壳，block_to_plain 的字符串出口）。
pub fn markdown_to_plain(markdown: &str) -> String {
    parse(markdown)
        .iter()
        .map(block_to_plain)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// 行内文本 → 纯文本（标题剥壳出口）：剥 `**`/`*`/`<u>`/`==` 等行内标记，但不剥行首 `#` 级联。
pub fn inline_to_plain_text(source: &str) -> String {
    inline_to_plain(&parse_inline(source))
}

/// 标题文本 → 纯文本（顶栏标题 / 列表预览标题专用）：在行内剥壳之上额外剥一层行首 `#` 前缀。
pub fn title_to_plain(source: &str) -> String {
    let plain = inline_to_plain(&parse_inline(source));
    match title_prefix().find(&plain).ok().flatten() {
        Some(m) => plain[m.end()..].to_string(),
        None => plain,
    }
}

/// AI 回写防冲刷护城河（rich-text-media.md §7）：对比原 md 与 AI 产出 md 的**行内媒体块 url
/// 集合**，返回原文有而产出丢失的 url。非空 = AI 自作主张删掉了用户媒体资产，动作层必须拒绝整替。
pub fn lost_media_urls(original: &str, incoming: &str) -> Vec<String> {
    let orig = media_urls_of(original);
    if orig.is_empty() {
        return Vec::new();
    }
    let inc = media_urls_of(incoming);
    orig.difference(&inc).cloned().collect()
}

fn media_urls_of(md: &str) -> HashSet<String> {
    let mut set = HashSet::new();
    fn walk(blocks: &[RichBlock], set: &mut HashSet<String>) {
        for b in blocks {
            match b {
                RichBlock::Image { url, .. } => {
                    set.insert(url.clone());
                }
                RichBlock::Audio { url, .. } => {
                    set.insert(url.clone());
                }
                RichBlock::Video { url, .. } => {
                    set.insert(url.clone());
                }
                RichBlock::Quote(children) => walk(children, set),
                _ => {}
            }
        }
    }
    walk(&parse(md), &mut set);
    set
}

/// 扫描 markdown 中的**待办行纯文本**（与渲染同源：ListBlock 中 `done != null` 的项，
/// 含列表式与裸 `[ ]` 行）。返回 inline_to_plain 同款纯文本并 trim。
pub fn scan_todo_texts(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(blocks: &[RichBlock], out: &mut Vec<String>) {
        for b in blocks {
            match b {
                RichBlock::List { items, .. } => {
                    for it in items {
                        if it.done.is_some() {
                            let text = it
                                .lines
                                .iter()
                                .map(|l| inline_to_plain(l))
                                .collect::<Vec<_>>()
                                .join("\n")
                                .trim()
                                .to_string();
                            out.push(text);
                        }
                    }
                }
                RichBlock::Quote(children) => walk(children, out),
                _ => {}
            }
        }
    }
    walk(&parse(markdown), &mut out);
    out
}

/// AI 写入归一层（rich-text-gfm.md §2 层2，命令入口唯一收口）：对**子集外**语法做语义映射
/// （找最接近的子集形式降级，不做「剥成纯文本」），并集外降级进 [AiNormalizeResult.notes]。
///
/// 当前子集外映射：
/// 1. 脚注 `[^id]` + 定义 `[^id]: text`（含后续缩进续行）→ 括号注 `（text）`，定义行作 meta 剥离；
/// 2. 子集外结构（HTML 标签等）检测命中 → R1 note「已降级为段落」。
/// 未定义脚注 `[^id]` 留字面 + note「存在未定义脚注标记，已保留原样」（禁止静默丢格式）。
pub fn normalize_ai_markdown(markdown: &str) -> AiNormalizeResult {
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut defs: HashMap<String, String> = HashMap::new();
    let mut removed: HashSet<usize> = HashSet::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some(m) = footnote_def().captures(lines[i]).ok().flatten() {
            let id = m.grp(1).unwrap().to_lowercase();
            let mut buf = vec![m.grp(2).unwrap().trim().to_string()];
            removed.insert(i);
            // 续行：后续以空白缩进的行并入同一脚注正文（GFM 脚注续行约定）
            let mut j = i + 1;
            while j < lines.len() && indent_cont().is_match(lines[j]).unwrap() {
                buf.push(lines[j].trim().to_string());
                removed.insert(j);
                j += 1;
            }
            defs.insert(id, buf.join(" "));
            i = j;
        } else {
            i += 1;
        }
    }

    // 剥离定义行 + 续行；行尾换行保留为空行，解析器忽略空行，不丢相邻内容。
    let out_lines: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(idx, _)| !removed.contains(idx))
        .map(|(_, l)| *l)
        .collect();
    let mut out = out_lines.join("\n");

    // 内联 `[^id]` → `（text）`；未定义则留字面并记 orphan（禁止静默丢格式）。
    let mut orphan = false;
    out = replace_footnotes(&out, &defs, &mut orphan);

    // 子集外结构降级告知（R1）。
    let mut notes: Vec<String> = Vec::new();
    if has_unsupported_gfm(&out) {
        notes.push(
            "检测到未支持的 GFM 语法（如 HTML 标签），已降级为普通段落；\
             建议改用纯文本、表格或列表等已支持格式"
                .to_string(),
        );
    }
    if !defs.is_empty() {
        notes.push("脚注已转为括号注".to_string());
    }
    if orphan {
        notes.push("存在未定义脚注标记，已保留原样".to_string());
    }
    AiNormalizeResult {
        markdown: out,
        notes,
    }
}

fn replace_footnotes(out: &str, defs: &HashMap<String, String>, orphan: &mut bool) -> String {
    let re = footnote_inline();
    let mut result = String::new();
    let mut last = 0;
    for caps in re.captures_iter(out) {
        let caps = match caps {
            Ok(c) => c,
            Err(_) => continue,
        };
        let m = match caps.get(0) {
            Some(mm) => mm,
            None => continue,
        };
        result.push_str(&out[last..m.start()]);
        let id = caps.grp(1).unwrap_or("").to_lowercase();
        match defs.get(&id) {
            Some(text) => result.push_str(&format!("（{text}）")),
            None => {
                *orphan = true;
                result.push_str(m.as_str());
            }
        }
        last = m.end();
    }
    result.push_str(&out[last..]);
    result
}

/// 检测子集外、会被解析器降级为段落的 GFM 语法（仅取低误报信号）：行内 HTML 标签
/// `<…>`（跳过敏感代码围栏）。数学式 `$$…$$`/`$…$` 已是一等公民，不再判为不支持。
fn has_unsupported_gfm(markdown: &str) -> bool {
    let mut in_fence = false;
    for line in markdown.split('\n') {
        if line.trim().starts_with("```") || line.trim().starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if html_unsupported().is_match(line).unwrap() {
            return true;
        }
    }
    false
}
