//! 编译期正则访问器（lazy，进程内单例）。全部使用 fancy-regex 以支持回溯与反向引用
//! （行内标记的 `\5`/`\7` 反向引用依赖此能力），且与 Dart `RegExp` 语义对齐。
//!
//! 每个访问器持有自己的 `OnceLock`，首次调用时编译，后续直接返回单例。

use fancy_regex::Regex;
use std::sync::OnceLock;

macro_rules! lazy_re {
    ($name:ident, $pattern:expr) => {
        pub(crate) fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            R.get_or_init(|| Regex::new($pattern).expect("invalid regex pattern"))
        }
    };
}

lazy_re!(fence, r"^```(\w*)\s*$");
lazy_re!(divider, r"^\s*(?:---|\*\*\*|___)\s*$");
lazy_re!(heading, r"^(#{1,6})\s+(.*)$");
lazy_re!(quote, r"^\s*>\s?");
lazy_re!(bullet, r"^\s*[-*+]\s+(.*)$");
lazy_re!(ordered, r"^\s*\d+[.)]\s+(.*)$");
lazy_re!(image_line, r"^!\[([^\]]*)\]\(([^)]*)\)\s*$");
lazy_re!(link_line, r"^\[([^\]]*)\]\(([^)]*)\)\s*$");
/// 列表项里的待办标记：`[ ]` / `[x]`（裸待办行）。
lazy_re!(todo, r"^\[([ xX])\]\s+(.*)$");
/// 引用定义 `[id]: url`（R3，id 大小写不敏感）。
lazy_re!(ref_def, r"^\s*\[([^\]]+)\]:\s*(\S+)");
/// 引用链接 `[text][id]`（R3 解析）。
lazy_re!(ref_link, r"\[([^\]]*)\]\[([^\]]*)\]");
/// 行内合并正则（与 Dart 单一事实源严格一致，分组号见 Dart §3.6 ②）：
/// 转义 → 自动链接 → 粗斜体 → 粗体 → 斜体 → 下划线 → 删除线 → 高亮 → 链接。
/// 用 `[\s\S]` 代替 `.` 以等价 Dart 的 dotAll（`fancy-regex` 对 `(?s)` 支持不稳）。
lazy_re!(
    inline,
    r"\\([\\`*_\[$])|(https?://[^\s<>()]+|www\.[^\s<>()]+|mailto:[^\s<>()]+)|(\*\*\*)([\s\S]+?)\*\*\*|(\*\*|__)([\s\S]+?)\5|(\*|_)([\s\S]+?)\7|<u>([\s\S]+?)</u>|~~([\s\S]+?)~~|==([^\s=]+(?:\s+[^\s=]+)*)==|!?\[([^\]]*)\]\(([^)]*)\)"
);
/// 脚注定义收集（行首 `^\[\^id\]: text`，含后续缩进续行）。
lazy_re!(footnote_def, r"^\s*\[\^([^\]]+)\]:\s*(.*)$");
/// 脚注内联 `[^id]`。
lazy_re!(footnote_inline, r"\[\^([^\]]+)\]");
/// 行内 HTML 标签（用于子集外语法降级告知）。排除 GFM 角括号自动链接。
lazy_re!(html_unsupported, r"<(?!https?://|mailto:)[a-zA-Z/!][^>\n]*>");
/// 标题纯文本再剥一层行首 `#` 前缀。
lazy_re!(title_prefix, r"^#{1,6}\s+");
/// 表格分隔线单元格校验：`^[\s:\-]+$`。
lazy_re!(table_sep_cell, r"^[\s:\-]+$");
/// 续行判定：后续以空白缩进的行（GFM 脚注/定义续行）。
lazy_re!(indent_cont, r"^[ \t]+\S");

/// fancy-regex 的 `Captures` 取分组用 `.get(i).as_str()`，没有 `.group(i)`。
/// 用 trait 扩展统一为 `grp(i)`，调用处与 Dart 的 `m.group(i)` 一一对应。
/// （`find_iter` 返回的 `Match` 不含子组，子组必须走 `captures_iter` 的 `Captures`。）
pub(crate) trait CaptureGroup {
    fn grp(&self, i: usize) -> Option<&str>;
}

impl<'t> CaptureGroup for fancy_regex::Captures<'t> {
    fn grp(&self, i: usize) -> Option<&str> {
        self.get(i).map(|m| m.as_str())
    }
}
