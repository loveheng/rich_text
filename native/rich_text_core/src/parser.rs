//! Markdown 子集解析器实现（rich_text 抽象库的具体实现）。
//!
//! 与 Dart `markdown_subset.dart` 1:1 镜像：行内 code/math 双扫描（防回溯）、
//! 块解析责任链、引用链接 R3、表格、块级数学、列表续行、引用递归、媒体行识别。
//! 永不抛异常，永不丢内容（降级铁律）。

use crate::ast::*;
use crate::media::classify_media_url;
use crate::regexes::*;
use std::collections::HashMap;

/// 转义还原：`\X` → 字面 X（含 `\$` → 字面 `$`）。
pub(crate) fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let n = b.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if b[i] == b'\\' && i + 1 < n {
            let c = b[i + 1];
            if matches!(c, b'\\' | b'`' | b'*' | b'_' | b'[' | b']') {
                out.push(c as char);
                i += 2;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Markdown 子集 → 块树。
pub fn parse(markdown: &str) -> Vec<RichBlock> {
    let normalized = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();

    // R3 第一遍：收集引用定义 `[id]: url`（id 大小写不敏感），定义行作 meta 剥离，不渲染残壳。
    let mut ref_defs: HashMap<String, String> = HashMap::new();
    let mut body: Vec<String> = Vec::new();
    for line in &lines {
        if let Some(m) = ref_def().captures(line).ok().flatten() {
            let id = m.grp(1).unwrap().to_lowercase();
            let url = m.grp(2).unwrap().to_string();
            ref_defs.insert(id, url);
        } else {
            body.push(line.clone());
        }
    }

    // R3 第二遍：已识别的 `[text][id]` 重写为 `[text](url)`；孤立 `[text][id]`（id 无定义）留字面。
    let resolved = if ref_defs.is_empty() {
        body.join("\n")
    } else {
        resolve_ref_links(&body.join("\n"), &ref_defs)
    };
    parse_blocks(resolved.split('\n').map(|s| s.to_string()).collect())
}

/// R3：`[label][id]` → `[label](url)`；id 未定义则保留原串。
fn resolve_ref_links(text: &str, defs: &HashMap<String, String>) -> String {
    let re = ref_link();
    let mut result = String::new();
    let mut last = 0;
    for caps in re.captures_iter(text) {
        let caps = match caps {
            Ok(c) => c,
            Err(_) => continue,
        };
        let m = match caps.get(0) {
            Some(mm) => mm,
            None => continue,
        };
        result.push_str(&text[last..m.start()]);
        let label = caps.grp(1).unwrap_or("").to_string();
        let id = caps.grp(2).unwrap_or("").to_lowercase();
        match defs.get(&id) {
            Some(url) => result.push_str(&format!("[{label}]({url})")),
            None => result.push_str(m.as_str()),
        }
        last = m.end();
    }
    result.push_str(&text[last..]);
    result
}

/// 流式行读取器（替代手写下标循环）。`lines` 为整篇行切片，`peek(offset)` 供表格二行前瞻。
struct LineReader<'a> {
    lines: &'a [String],
    index: usize,
}

impl<'a> LineReader<'a> {
    fn new(lines: &'a [String]) -> Self {
        Self { lines, index: 0 }
    }
    fn is_done(&self) -> bool {
        self.index >= self.lines.len()
    }
    fn peek(&self, offset: usize) -> Option<&'a str> {
        let at = self.index + offset;
        if at < self.lines.len() {
            Some(&self.lines[at])
        } else {
            None
        }
    }
    fn advance(&mut self) -> &'a str {
        let l = &self.lines[self.index];
        self.index += 1;
        l
    }
    fn advance_by(&mut self, c: usize) {
        self.index += c;
    }
}

/// 块解析责任链调度。段落永远垫底兜底。
fn parse_blocks(lines: Vec<String>) -> Vec<RichBlock> {
    let mut reader = LineReader::new(&lines);
    let mut blocks = Vec::new();
    while !reader.is_done() {
        // 空行是块间隙：跳过，且绝不并入段落。
        if reader.peek(0).unwrap().trim().is_empty() {
            reader.advance();
            continue;
        }
        let start = reader.index;
        let head = reader.peek(0).unwrap();
        if fence().is_match(head).unwrap() {
            blocks.push(parse_fence(&mut reader));
        } else if divider().is_match(head).unwrap() {
            blocks.push(parse_divider(&mut reader));
        } else if head.trim().starts_with("$$") {
            blocks.push(parse_math_block(&mut reader));
        } else if is_table_start(reader.lines, reader.index) {
            blocks.push(parse_table(&mut reader));
        } else if heading().is_match(head).unwrap() {
            blocks.push(parse_heading(&mut reader));
        } else if quote().is_match(head).unwrap() {
            blocks.push(parse_quote(&mut reader));
        } else if is_list_start(head) {
            blocks.push(parse_list(&mut reader));
        } else if image_line().is_match(head).unwrap() {
            blocks.push(parse_image_line(&mut reader));
        } else if is_media_link(head) {
            blocks.push(parse_media_link(&mut reader));
        } else {
            blocks.push(parse_paragraph(&mut reader));
        }
        // 死循环护栏：命中却未推进读针则强制前进。
        if reader.index == start {
            reader.advance();
        }
    }
    blocks
}

/// 块起始判定（供 [parse_paragraph] / [is_continuation] 调用）。
fn is_block_start(line: &str) -> bool {
    fence().is_match(line).unwrap()
        || divider().is_match(line).unwrap()
        || heading().is_match(line).unwrap()
        || quote().is_match(line).unwrap()
        || bullet().is_match(line).unwrap()
        || ordered().is_match(line).unwrap()
        || image_line().is_match(line).unwrap()
        || (link_line().is_match(line).unwrap()
            && classify_media_url(&link_url(line)) != MediaSuffix::Unknown)
        || line.trim().starts_with("$$")
}

fn link_url(line: &str) -> String {
    link_line()
        .captures(line)
        .ok()
        .flatten()
        .and_then(|m| m.grp(2).map(|s| s.to_string()))
        .unwrap_or_default()
}

// ---- 表格（GFM）----

fn is_table_start(lines: &[String], i: usize) -> bool {
    if i + 1 >= lines.len() {
        return false;
    }
    is_row_line(&lines[i]) && is_separator_line(&lines[i + 1])
}

fn is_row_line(line: &str) -> bool {
    line.trim().contains('|')
}

fn is_separator_line(line: &str) -> bool {
    let t = line.trim();
    if !t.contains('|') {
        return false;
    }
    let no_pipes = t.replace('|', "");
    if !no_pipes.contains('-') {
        return false;
    }
    table_sep_cell().is_match(&no_pipes).unwrap()
}

fn split_row(line: &str) -> Vec<String> {
    let mut t = line.trim().to_string();
    if t.starts_with('|') {
        t = t[1..].to_string();
    }
    if t.ends_with('|') {
        t = t[..t.len() - 1].to_string();
    }
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn align_of(sep: &str) -> TableAlign {
    let t = sep.trim();
    let left = t.starts_with(':');
    let right = t.ends_with(':');
    if left && right {
        TableAlign::Center
    } else if right {
        TableAlign::Right
    } else {
        TableAlign::Left
    }
}

// ---- 各块语法 ----

fn parse_fence(r: &mut LineReader) -> RichBlock {
    let open = r.advance();
    let lang = fence()
        .captures(open)
        .ok()
        .flatten()
        .and_then(|m| m.grp(1).map(|s| s.to_string()))
        .unwrap_or_default();
    let mut buf = Vec::new();
    while !r.is_done() && !fence().is_match(r.peek(0).unwrap()).unwrap() {
        buf.push(r.advance().to_string());
    }
    if !r.is_done() {
        r.advance(); // 消费闭合围栏（缺失时按到文末，旧版行为）
    }
    RichBlock::CodeBlock {
        code: buf.join("\n"),
        language: if lang.is_empty() { None } else { Some(lang) },
    }
}

fn parse_divider(r: &mut LineReader) -> RichBlock {
    r.advance();
    RichBlock::Divider
}

fn parse_math_block(r: &mut LineReader) -> RichBlock {
    let first = r.advance().to_string();
    if first.trim() != "$$" && math_has_closing(&first) {
        return math_block_of(&[first.clone()]);
    }
    let mut buf = vec![first];
    while !r.is_done() {
        let line = r.advance().to_string();
        buf.push(line.clone());
        if math_has_closing(&line) {
            break;
        }
    }
    math_block_of(&buf)
}

fn math_has_closing(line: &str) -> bool {
    if line.trim() == "$$" {
        return true;
    }
    let first = match line.find("$$") {
        Some(f) => f,
        None => return false,
    };
    let close = match line[first + 2..].find("$$") {
        Some(p) => first + 2 + p,
        None => return false,
    };
    line[close + 2..].trim().is_empty()
}

fn math_block_of(buf: &[String]) -> RichBlock {
    let text = buf.join("\n");
    let open = text.find("$$").unwrap_or(0);
    let close = text[open + 2..].find("$$").map(|p| open + 2 + p);
    match close {
        Some(c) => RichBlock::MathBlock(text[open + 2..c].to_string()),
        // 无闭合降级段落（保留 `$$` 字面，降级铁律不丢内容）。
        None => RichBlock::Paragraph(parse_inline(&text)),
    }
}

fn parse_table(r: &mut LineReader) -> RichBlock {
    let (block, consumed) = parse_table_impl(r.lines, r.index);
    r.advance_by(consumed);
    block
}

fn parse_table_impl(lines: &[String], i: usize) -> (RichBlock, usize) {
    let header = split_row(&lines[i]);
    let seps = split_row(&lines[i + 1]);
    let align: Vec<TableAlign> = seps.iter().map(|s| align_of(s)).collect();
    let mut rows = Vec::new();
    let mut j = i + 2;
    while j < lines.len() && is_row_line(&lines[j]) {
        let cells = split_row(&lines[j]);
        if cells.len() != header.len() {
            break; // 列数不一致则停止（防吞后续）
        }
        rows.push(cells);
        j += 1;
    }
    (
        RichBlock::Table {
            header,
            rows,
            align: Some(align),
        },
        j - i,
    )
}

fn parse_heading(r: &mut LineReader) -> RichBlock {
    let m = heading().captures(r.advance()).ok().flatten().unwrap();
    let level = m.grp(1).unwrap().len() as u8;
    let inline = parse_inline(m.grp(2).unwrap());
    RichBlock::Heading { level, inline }
}

fn parse_quote(r: &mut LineReader) -> RichBlock {
    let mut buf = Vec::new();
    while !r.is_done() && quote().is_match(r.peek(0).unwrap()).unwrap() {
        let line = r.advance();
        buf.push(quote().replace(line, "").to_string());
    }
    // 剥离 `>` 后递归走同一个 parse_blocks，保证嵌套结构与行内口径一致。
    RichBlock::Quote(parse_blocks(buf))
}

fn parse_list(r: &mut LineReader) -> RichBlock {
    let first = r.peek(0).unwrap().to_string();
    let is_ordered = ordered().is_match(&first).unwrap();
    let mut items: Vec<ListItem> = Vec::new();
    while !r.is_done() {
        let line = r.peek(0).unwrap().to_string();
        // 续行：已有前序项且本行是缩进续行 → 并入当前项。
        if !items.is_empty() && is_continuation(r, &line) {
            let cont = line.trim_start_matches(' ').to_string();
            items.last_mut().unwrap().lines.push(parse_inline(&cont));
            r.advance();
            continue;
        }
        let m = if is_ordered {
            ordered()
                .captures(&line)
                .ok()
                .flatten()
                .or_else(|| bullet().captures(&line).ok().flatten())
        } else {
            bullet().captures(&line).ok().flatten()
        };
        if let Some(m) = m {
            let content = m.grp(1).unwrap().to_string();
            items.push(list_item_impl(&content));
            r.advance();
            continue;
        }
        // 裸待办行 `[ ] xxx` 归入列表（并入前序列表），传原始行。
        if todo().is_match(&line).unwrap() {
            items.push(list_item_impl(&line));
            r.advance();
            continue;
        }
        break;
    }
    RichBlock::List {
        ordered: is_ordered,
        items,
    }
}

fn list_item_impl(content: &str) -> ListItem {
    // 与 Dart `_listItemImpl` 对齐：无论列表项内容来自 bullet/ordered 剥离还是裸待办行，
    // 都先查待办标记 `[ ]`/`[x]`（如 `- [ ] x` 的 content 正是 `[ ] x`）。命中则归待办，
    // 未命中才是普通列表项。
    if let Some(m) = todo().captures(content).ok().flatten() {
        let done = m.grp(1).unwrap() != " ";
        let inner = m.grp(2).unwrap().to_string();
        return ListItem {
            lines: vec![parse_inline(&inner)],
            done: Some(done),
        };
    }
    ListItem {
        lines: vec![parse_inline(content)],
        done: None,
    }
}

/// 续行判定：≥2 空格缩进、非块起始、非新列表项（含裸待办）才并入前序项。
fn is_continuation(r: &LineReader, line: &str) -> bool {
    if line.trim().is_empty() {
        return false;
    }
    if !line.starts_with("  ") {
        return false; // 必须缩进（≥2 空格）
    }
    if is_block_start(line) {
        return false; // 标题/引用/表格等
    }
    if is_table_start(r.lines, r.index) {
        return false;
    }
    if bullet().is_match(line).unwrap()
        || ordered().is_match(line).unwrap()
        || todo().is_match(line).unwrap()
    {
        return false; // 新列表项（含裸待办）另起项
    }
    true
}

fn is_list_start(line: &str) -> bool {
    bullet().is_match(line).unwrap()
        || ordered().is_match(line).unwrap()
        || todo().is_match(line).unwrap()
}

fn parse_image_line(r: &mut LineReader) -> RichBlock {
    let m = image_line().captures(r.advance()).ok().flatten().unwrap();
    RichBlock::Image {
        url: m.grp(2).unwrap().to_string(),
        alt: unescape(m.grp(1).unwrap()),
    }
}

fn is_media_link(line: &str) -> bool {
    if let Some(m) = link_line().captures(line).ok().flatten() {
        let url = m.grp(2).unwrap();
        return classify_media_url(url) != MediaSuffix::Unknown;
    }
    false
}

fn parse_media_link(r: &mut LineReader) -> RichBlock {
    let m = link_line().captures(r.advance()).ok().flatten().unwrap();
    let url = m.grp(2).unwrap().to_string();
    let label = unescape(m.grp(1).unwrap());
    match classify_media_url(&url) {
        MediaSuffix::AudioPlayable | MediaSuffix::AudioDegrade => {
            RichBlock::Audio { url, label }
        }
        _ => RichBlock::Video { url, label },
    }
}

fn parse_paragraph(r: &mut LineReader) -> RichBlock {
    let mut buf = Vec::new();
    buf.push(r.advance().to_string());
    while !r.is_done() {
        let line = r.peek(0).unwrap().to_string();
        if line.trim().is_empty() {
            break;
        }
        if is_block_start(&line) {
            break;
        }
        if is_table_start(r.lines, r.index) {
            break;
        }
        buf.push(r.advance().to_string());
    }
    RichBlock::Paragraph(parse_inline(&buf.join("\n")))
}

// ---------- 行内解析 ----------

/// 行内码双扫描：先定位连续反引号定界符（`+），再找同长度闭合——线性时间，无回溯。
pub(crate) fn extract_code(text: &str) -> Vec<(usize, usize, String)> {
    let b = text.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let mut k = 0;
        while i + k < n && b[i + k] == b'`' {
            k += 1;
        }
        let open_start = i;
        i += k;
        let mut j = i;
        let mut found = false;
        while j < n {
            if b[j] == b'`' {
                let mut m = 0;
                while j + m < n && b[j + m] == b'`' {
                    m += 1;
                }
                if m == k {
                    let content = text[open_start + k..j].to_string();
                    out.push((open_start, j + k, content));
                    i = j + k;
                    found = true;
                    break;
                }
                j += m;
                continue;
            }
            j += 1;
        }
        if !found {
            i = open_start + k;
        }
    }
    out
}

/// 行内数学双扫描：定位行内 `$...$`（单行、内容不含 `$`、非 `$$` 块级），线性时间无回溯。
/// 边缘规则排除货币歧义（数学扩展约定）：开口 `$` 前为数字、后为空白/数字、闭合 `$` 前
/// 为空白/后为数字，均非数学。
pub(crate) fn extract_math(text: &str) -> Vec<(usize, usize, String)> {
    let b = text.as_bytes();
    let n = b.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if b[i] != b'$' {
            i += 1;
            continue;
        }
        if i > 0 && b[i - 1] == b'\\' {
            i += 1;
            continue;
        }
        if i + 1 < n && b[i + 1] == b'$' {
            i += 2;
            continue;
        }
        if i + 1 >= n {
            i += 1;
            continue;
        }
        let after = b[i + 1];
        if after == b' ' || after == b'\t' || after == b'\n' || after.is_ascii_digit() {
            i += 1;
            continue;
        }
        if i > 0 && b[i - 1].is_ascii_digit() {
            i += 1;
            continue;
        }
        let open_start = i;
        let inner_start = i + 1;
        let mut j = inner_start;
        let mut found = false;
        while j < n {
            let c = b[j];
            if c == b'\n' {
                break; // 行内数学不跨行
            }
            if c == b'\\' {
                j += 2; // 跳过转义序列（如公式内 \$）
                continue;
            }
            if c == b'$' {
                let before_ok = j > inner_start && b[j - 1] != b' ' && b[j - 1] != b'\t';
                let after_ok = j + 1 >= n || !b[j + 1].is_ascii_digit();
                if before_ok && after_ok {
                    let content = text[inner_start..j].to_string();
                    out.push((open_start, j + 1, content));
                    i = j + 1;
                    found = true;
                    break;
                }
                break; // 闭合不满足边缘规则：放弃本处，避免吞掉后续合法公式
            }
            j += 1;
        }
        if !found {
            i = open_start + 1;
        }
    }
    out
}

/// 行内解析实现：匹配顺序即优先级；返回前合并相邻纯文本节点（规范形，
/// 保证 parse→serialize→parse 块树逐节点相等）。
pub fn parse_inline(text: &str) -> Vec<InlineNode> {
    if text.is_empty() {
        return vec![InlineNode::Text(String::new())];
    }
    let code = extract_code(text);
    let math = extract_math(text);
    let overlaps_code = |s: usize, e: usize| code.iter().any(|c| s < c.1 && c.0 < e);
    let overlaps_math = |s: usize, e: usize| math.iter().any(|m| s < m.1 && m.0 < e);

    struct Ev {
        start: usize,
        end: usize,
        code: Option<String>,
        math: Option<String>,
        groups: Option<Vec<Option<String>>>,
    }

    let mut events: Vec<Ev> = Vec::new();
    for c in &code {
        events.push(Ev {
            start: c.0,
            end: c.1,
            code: Some(c.2.clone()),
            math: None,
            groups: None,
        });
    }
    for m in &math {
        if overlaps_code(m.0, m.1) {
            continue;
        }
        events.push(Ev {
            start: m.0,
            end: m.1,
            code: None,
            math: Some(m.2.clone()),
            groups: None,
        });
    }
    let re = inline();
    for cres in re.captures_iter(text) {
        let caps = match cres {
            Ok(c) => c,
            Err(_) => continue,
        };
        let whole = match caps.get(0) {
            Some(w) => w,
            None => continue,
        };
        let (s, e) = (whole.start(), whole.end());
        if overlaps_code(s, e) || overlaps_math(s, e) {
            continue;
        }
        let groups: Vec<Option<String>> =
            (0..=13).map(|i| caps.get(i).map(|x| x.as_str().to_string())).collect();
        events.push(Ev {
            start: s,
            end: e,
            code: None,
            math: None,
            groups: Some(groups),
        });
    }
    events.sort_by_key(|ev| ev.start);

    let mut out: Vec<InlineNode> = Vec::new();
    let mut pos = 0;
    for ev in &events {
        if ev.start > pos {
            out.push(InlineNode::Text(text[pos..ev.start].to_string()));
        }
        if let Some(math) = &ev.math {
            out.push(InlineNode::Math(math.clone()));
        } else if let Some(code) = &ev.code {
            out.push(InlineNode::Code(code.clone()));
        } else if let Some(g) = &ev.groups {
            out.push(inline_node_from_match(g));
        }
        pos = ev.end;
    }
    if pos < text.len() {
        out.push(InlineNode::Text(text[pos..].to_string()));
    }
    if out.is_empty() {
        vec![InlineNode::Text(text.to_string())]
    } else {
        merge_text(out)
    }
}

/// 行内正则命中 → [InlineNode]（分组号对应 [inline] 正则）。
fn inline_node_from_match(g: &[Option<String>]) -> InlineNode {
    if let Some(s) = &g[1] {
        return InlineNode::Text(s.clone()); // 转义
    }
    if let Some(s) = &g[2] {
        return InlineNode::Link {
            label: s.clone(),
            url: s.clone(),
            autolink: true,
        };
    }
    if let Some(inner) = &g[4] {
        return InlineNode::Strong(vec![InlineNode::Em(parse_inline(inner))]);
    }
    if let Some(inner) = &g[6] {
        return InlineNode::Strong(parse_inline(inner));
    }
    if let Some(inner) = &g[8] {
        return InlineNode::Em(parse_inline(inner));
    }
    if let Some(inner) = &g[9] {
        return InlineNode::Underline(parse_inline(inner));
    }
    if let Some(inner) = &g[10] {
        return InlineNode::Strikethrough(parse_inline(inner));
    }
    if let Some(inner) = &g[11] {
        return InlineNode::Highlight(parse_inline(inner));
    }
    if let Some(label) = &g[12] {
        let url = g[13].clone().unwrap_or_default();
        return InlineNode::Link {
            label: unescape(label),
            url,
            autolink: false,
        };
    }
    InlineNode::Text(g[0].clone().unwrap_or_default())
}

fn merge_text(nodes: Vec<InlineNode>) -> Vec<InlineNode> {
    let mut out: Vec<InlineNode> = Vec::new();
    for n in nodes {
        if let InlineNode::Text(t) = &n {
            if let Some(InlineNode::Text(last)) = out.last_mut() {
                last.push_str(t);
                continue;
            }
        }
        out.push(n);
    }
    out
}

// ---------- 行内样式 spans（block-format-input.md §4 样式化编辑层地基） ----------

/// 源 md 行内文本的样式化投影：`plain`（标记剥除后的纯文本）+ `runs`（UTF-8 字节坐标）。
///
/// 同源护栏：[plain] 必须与 `inline_to_plain(parse_inline(source))` 逐字符一致；
/// 分支顺序/转义/不闭合降级语义与 [parse_inline] 镜像，改一处必改另一处。
pub fn inline_spans_of(source: &str) -> InlineSpans {
    let mut buf = String::new();
    let mut runs: Vec<InlineRun> = Vec::new();
    let mut active: Vec<InlineMark> = Vec::new();
    spans_walk(source, &mut buf, &mut runs, &mut active);
    InlineSpans { plain: buf, runs }
}

fn spans_walk(
    text: &str,
    buf: &mut String,
    runs: &mut Vec<InlineRun>,
    active: &mut Vec<InlineMark>,
) {
    let code = extract_code(text);
    let math = extract_math(text);
    let overlaps_code = |s: usize, e: usize| code.iter().any(|c| s < c.1 && c.0 < e);
    let overlaps_math = |s: usize, e: usize| math.iter().any(|m| s < m.1 && m.0 < e);

    struct Ev {
        start: usize,
        end: usize,
        code: Option<String>,
        math: Option<String>,
        groups: Option<Vec<Option<String>>>,
    }

    let mut events: Vec<Ev> = Vec::new();
    for c in &code {
        events.push(Ev {
            start: c.0,
            end: c.1,
            code: Some(c.2.clone()),
            math: None,
            groups: None,
        });
    }
    for m in &math {
        if overlaps_code(m.0, m.1) {
            continue;
        }
        events.push(Ev {
            start: m.0,
            end: m.1,
            code: None,
            math: Some(m.2.clone()),
            groups: None,
        });
    }
    let re = inline();
    for cres in re.captures_iter(text) {
        let caps = match cres {
            Ok(c) => c,
            Err(_) => continue,
        };
        let whole = match caps.get(0) {
            Some(w) => w,
            None => continue,
        };
        let (s, e) = (whole.start(), whole.end());
        if overlaps_code(s, e) || overlaps_math(s, e) {
            continue;
        }
        let groups: Vec<Option<String>> =
            (0..=13).map(|i| caps.get(i).map(|x| x.as_str().to_string())).collect();
        events.push(Ev {
            start: s,
            end: e,
            code: None,
            math: None,
            groups: Some(groups),
        });
    }
    events.sort_by_key(|ev| ev.start);

    let mut pos = 0;
    for ev in &events {
        if ev.start > pos {
            emit(buf, runs, active, &text[pos..ev.start], None);
        }
        if let Some(math) = &ev.math {
            active.push(InlineMark::Math);
            emit(buf, runs, active, math, None);
            active.pop();
        } else if let Some(code) = &ev.code {
            active.push(InlineMark::Code);
            emit(buf, runs, active, code, None);
            active.pop();
        } else if let Some(g) = &ev.groups {
            if let Some(s) = &g[1] {
                emit(buf, runs, active, s, None);
            } else if let Some(s) = &g[2] {
                active.push(InlineMark::Link);
                emit(buf, runs, active, s, Some(s.clone()));
                active.pop();
            } else if let Some(s) = &g[4] {
                active.push(InlineMark::Bold);
                active.push(InlineMark::Italic);
                spans_walk(s, buf, runs, active);
                active.pop();
                active.pop();
            } else if let Some(s) = &g[6] {
                active.push(InlineMark::Bold);
                spans_walk(s, buf, runs, active);
                active.pop();
            } else if let Some(s) = &g[8] {
                active.push(InlineMark::Italic);
                spans_walk(s, buf, runs, active);
                active.pop();
            } else if let Some(s) = &g[9] {
                active.push(InlineMark::Underline);
                spans_walk(s, buf, runs, active);
                active.pop();
            } else if let Some(s) = &g[10] {
                active.push(InlineMark::Strikethrough);
                spans_walk(s, buf, runs, active);
                active.pop();
            } else if let Some(s) = &g[11] {
                active.push(InlineMark::Highlight);
                spans_walk(s, buf, runs, active);
                active.pop();
            } else if let Some(s) = &g[12] {
                active.push(InlineMark::Link);
                emit(buf, runs, active, &unescape(s), Some(g[13].clone().unwrap_or_default()));
                active.pop();
            }
        }
        pos = ev.end;
    }
    if pos < text.len() {
        emit(buf, runs, active, &text[pos..], None);
    }
}

fn emit(
    buf: &mut String,
    runs: &mut Vec<InlineRun>,
    active: &[InlineMark],
    plain: &str,
    url: Option<String>,
) {
    if plain.is_empty() {
        return;
    }
    let start = buf.len();
    buf.push_str(plain);
    let end = buf.len();
    for m in active {
        runs.push(InlineRun {
            start,
            end,
            mark: *m,
            url: url.clone(),
        });
    }
}
