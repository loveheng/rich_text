//! rich_text_core — 单一原生核心（方案 B）。
//!
//! 本 crate 是 `rich_text` Markdown 子集解析/序列化/编辑原语的**唯一事实源**，
//! 用纯 Rust 实现（核心逻辑零运行时依赖）。Android(Kotlin)/iOS(Swift)/Dart(Flutter)
//! 各端通过 UniFFI 绑定（`rich_text_uniffi` crate）消费本核心，杜绝逐平台各写一份
//! 的 fork 漂移（评估文档 native-rewrite-eval.md §3 方案 C 已否决）。
//!
//! 设计 SSOT 在 goodshare 仓 `docs/design/`（content-pipeline / rich-text-* / ui-spec）。
//! 降级铁律：**宁可样式平，不可吞内容**——不认识的块一律降级为段落，绝不丢字符。
//! 坐标契约：行内 run 坐标统一为 **UTF-8 字节偏移**（Dart 版为 UTF-16 code unit，
//! 跨语言边界按字节统一，见 native-rewrite-eval.md §4.2）。

#![allow(unused_doc_comments)]

pub mod ast;
pub mod helpers;
pub mod inline;
pub mod media;
pub mod parser;
pub mod regexes;
pub mod serialize;

pub use ast::*;
pub use helpers::*;
pub use inline::*;
pub use media::*;
pub use parser::*;
pub use serialize::*;

/// 核心版本号（绑定层可读取，用于兼容性判断）。
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// `Vec<RichBlock>` → JSON 字符串（跨语言边界的序列化形态；方案 B §4 推荐）。
#[cfg(feature = "serde")]
pub fn blocks_to_json(blocks: &[RichBlock]) -> String {
    serde_json::to_string(blocks).expect("serialize blocks to json")
}

/// JSON 字符串 → `Vec<RichBlock>`（黄金语料 / 绑定层往返用）。
#[cfg(feature = "serde")]
pub fn blocks_from_json(json: &str) -> Result<Vec<RichBlock>, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_present() {
        assert!(!version().is_empty());
    }

    #[test]
    fn heading_and_inline() {
        let blocks = parse("# Hello **world**");
        assert_eq!(
            blocks,
            vec![RichBlock::Heading {
                level: 1,
                inline: vec![
                    InlineNode::Text("Hello ".into()),
                    InlineNode::Strong(vec![InlineNode::Text("world".into())]),
                ],
            }]
        );
    }

    #[test]
    fn round_trip_idempotent() {
        let md = "# T\n\n- [ ] todo one\n- [x] done\n\n> quote *italic*\n\n```rs\ncode\n```\n";
        let once = serialize_blocks(&parse(md));
        let twice = serialize_blocks(&parse(&once));
        assert_eq!(once, twice, "parse→serialize 必须幂等");
    }

    #[test]
    fn inline_spans_match_plain() {
        let s = inline_spans_of("a **b** c");
        assert_eq!(s.plain, "a b c");
        assert!(s.runs.iter().any(|r| r.mark == InlineMark::Bold));
    }

    #[test]
    fn normalize_footnote() {
        // 脚注定义行被剥离后保留其空行位（与 Dart 算法一致），故 trim 后比对。
        let r = normalize_ai_markdown("[^1]: hi\n\nsome [^1] text");
        assert_eq!(r.markdown.trim(), "some （hi） text");
        assert!(r.notes.contains(&"脚注已转为括号注".to_string()));
    }

    #[test]
    fn media_loss_detected() {
        let lost = lost_media_urls(
            "![a](x.png)\n\n![b](y.jpg)",
            "![a](x.png)",
        );
        assert_eq!(lost, vec!["y.jpg".to_string()]);
    }
}
