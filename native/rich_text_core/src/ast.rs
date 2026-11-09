//! AST 模型（块级 + 行内）。与 Dart `rich_text.dart` 1:1 镜像。
//!
//! 降级铁律：**宁可样式平，不可吞内容**——不认识的块一律降级为段落，绝不丢字符。
//! 坐标契约：行内 run 坐标统一为 **UTF-8 字节偏移**（Dart 版为 UTF-16 code unit，
//! 跨语言边界按字节统一，见 native-rewrite-eval.md §4.2）。

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InlineNode {
    Text(String),
    Strong(Vec<InlineNode>),
    Em(Vec<InlineNode>),
    Underline(Vec<InlineNode>),
    Code(String),
    Link { label: String, url: String, autolink: bool },
    Strikethrough(Vec<InlineNode>),
    Highlight(Vec<InlineNode>),
    Math(String),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RichBlock {
    Heading { level: u8, inline: Vec<InlineNode> },
    Paragraph(Vec<InlineNode>),
    Quote(Vec<RichBlock>),
    List { ordered: bool, items: Vec<ListItem> },
    CodeBlock { code: String, language: Option<String> },
    Divider,
    Image { url: String, alt: String },
    Audio { url: String, label: String },
    Video { url: String, label: String },
    MathBlock(String),
    Table { header: Vec<String>, rows: Vec<Vec<String>>, align: Option<Vec<TableAlign>> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TableAlign {
    Left,
    Center,
    Right,
}

/// 列表项。`done` 非 null 表示待办项（可勾选）。
///
/// `lines` 首行为 `- `/`1. `/`- [ ] ` 标记后的内容；后续元素为「续行」
/// （以 ≥2 空格缩进表示，解析层归入同一项）。单行项仅一个元素。
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListItem {
    pub lines: Vec<Vec<InlineNode>>,
    /// true=已完成 `[x]`，false=未完成 `[ ]`，None=普通列表项。
    pub done: Option<bool>,
}

impl ListItem {
    /// 首行内容（兼容旧单行调用；多行项请直接用 [lines]）。
    pub fn inline(&self) -> Vec<InlineNode> {
        match self.lines.first() {
            Some(l) => l.clone(),
            None => vec![InlineNode::Text(String::new())],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MediaSuffix {
    AudioPlayable,
    AudioDegrade,
    Video,
    Unknown,
}

/// 行内样式种类（编辑态样式化层用；渲染即 switch 出对应 TextStyle 增量）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InlineMark {
    Bold,
    Italic,
    Code,
    Underline,
    Link,
    Strikethrough,
    Highlight,
    Math,
}

impl InlineMark {
    /// 固定优先级顺序（bold > italic > code > underline > link），用于
    /// [crate::inline_nodes_of] 嵌套合成。
    pub fn all() -> &'static [InlineMark] {
        &[
            InlineMark::Bold,
            InlineMark::Italic,
            InlineMark::Code,
            InlineMark::Underline,
            InlineMark::Link,
            InlineMark::Strikethrough,
            InlineMark::Highlight,
            InlineMark::Math,
        ]
    }
}

/// 一个样式 run：**纯文本坐标** [start, end)（标记字符已剥除，UTF-8 字节偏移）。
/// 嵌套（如粗体含于下划线）以重叠 run 表达——渲染侧按字符合成样式。
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InlineRun {
    pub start: usize,
    pub end: usize,
    pub mark: InlineMark,
    /// 仅 link run 携带：plain 只含 label，url 由 run 载荷保全（数据无损防线）。
    pub url: Option<String>,
}

/// [crate::inline_spans_of] 的返回：纯文本 + 纯文本坐标的样式 run 列表。
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InlineSpans {
    pub plain: String,
    pub runs: Vec<InlineRun>,
}

/// 行级档位 run（速记所见即所得的行标题）：**纯文本坐标** [start, end)，
/// 恒与实际整行对齐（宿主侧 snap）。
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanLevelRun {
    pub start: usize,
    pub end: usize,
    /// 0 正文 / 1 一级标题 / 2 二级标题（0 不参与渲染，序列化侧消费）。
    pub level: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpanListKind {
    Bullet,
    Todo,
    Ordered,
}

/// 行列表 run：**纯文本坐标** [start, end)，覆盖一整行（不含行尾 `\n`）。
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanListRun {
    pub start: usize,
    pub end: usize,
    pub kind: SpanListKind,
    /// 待办勾选态（仅 [SpanListKind::Todo] 有意义）；渲染侧消费，不改正文。
    pub done: bool,
}

/// AI 写入归一结果（语义映射 + R1 note 载体）。
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AiNormalizeResult {
    pub markdown: String,
    pub notes: Vec<String>,
}
