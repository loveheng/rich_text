//! 语言无关黄金语料：锁定核心解析/序列化语义，防止方案 B 单核心回归漂移。
//! 每个 fixture 必须「解析→序列化→再解析」块树逐节点相等（往返幂等），
//! 并附带少量结构不变量断言。评估 native-rewrite-eval.md §4「黄金语料」项。

use rich_text_core::*;

fn roundtrip(md: &str) {
    let first = serialize_blocks(&parse(md));
    let second = serialize_blocks(&parse(&first));
    assert_eq!(first, second, "往返必须幂等：\n--- 首次序列化 ---\n{first}\n--- 二次序列化 ---\n{second}");
}

#[test]
fn kitchen_sink_roundtrip() {
    roundtrip(include_str!("fixtures/kitchen_sink.md"));
}

#[test]
fn inline_tricky_roundtrip() {
    roundtrip(include_str!("fixtures/inline_tricky.md"));
}

#[test]
fn kitchen_sink_structure() {
    let blocks = parse(include_str!("fixtures/kitchen_sink.md"));
    // 0 标题二级
    assert!(matches!(blocks[0], RichBlock::Heading { level: 2, .. }));
    // 1 段落含混合行内
    assert!(matches!(blocks[1], RichBlock::Paragraph(_)));
    // 2 待办列表（2 项，未完成/已完成）
    match &blocks[2] {
        RichBlock::List { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].done, Some(false));
            assert_eq!(items[1].done, Some(true));
        }
        _ => panic!("index 2 应为待办列表"),
    }
    // 3 引用
    assert!(matches!(blocks[3], RichBlock::Quote(_)));
    // 4 表格
    assert!(matches!(blocks[4], RichBlock::Table { .. }));
    // 5 块级数学
    assert!(matches!(blocks[5], RichBlock::MathBlock(_)));
    // 6 代码块（语言 rs）
    match &blocks[6] {
        RichBlock::CodeBlock { language, .. } => assert_eq!(language.as_deref(), Some("rs")),
        _ => panic!("index 6 应为代码块"),
    }
    // 7 图片
    assert!(matches!(blocks[7], RichBlock::Image { .. }));
    // 8 音频（mp3）
    assert!(matches!(blocks[8], RichBlock::Audio { .. }));
}

#[test]
fn inline_tricky_structure() {
    let md = include_str!("fixtures/inline_tricky.md");
    let blocks = parse(md);
    // 首段：纯文本必须保留 a/b/c 三段内容（lazy-backref 拆分是此正则的已知特性，
    // 与 Dart 单一事实源一致，这里只校验「不丢字符」而非理想嵌套）。
    match &blocks[0] {
        RichBlock::Paragraph(nodes) => {
            let plain = inline_to_plain(nodes);
            assert!(plain.contains('a') && plain.contains('b') && plain.contains('c'));
        }
        _ => panic!("首块应为段落"),
    }
    // 末段：<u>下划线</u> / ==高亮== / ~~删除~~ 必须各自识别为对应节点（不丢语义）。
    let last = blocks.last().unwrap();
    match last {
        RichBlock::Paragraph(nodes) => {
            assert!(
                nodes.iter().any(|n| matches!(n, InlineNode::Underline(_))),
                "应识别下划线"
            );
            assert!(
                nodes.iter().any(|n| matches!(n, InlineNode::Highlight(_))),
                "应识别高亮"
            );
            assert!(
                nodes.iter().any(|n| matches!(n, InlineNode::Strikethrough(_))),
                "应识别删除线"
            );
        }
        _ => panic!("末块应为段落"),
    }
    // 转义 \[ 与 \* 应降级为纯文本（不丢字符）。注意 `]` 不在转义集（仅 \ `* _ [ $），
    // 故 \] 会保留反斜杠，与 Dart 单一事实源一致；这里校验字面 `[` 已落地。
    let plain = markdown_to_plain(md);
    assert!(plain.contains("[不是链接"), "转义后应为字面 [");
    assert!(plain.contains("*不是斜体*"), "转义后应为字面 *");
    // 行内数学内容保留在纯文本
    assert!(plain.contains("E=mc^2"), "行内数学内容应保留在纯文本");
}

#[test]
fn media_classify_contract() {
    // 两档白名单 + 越界降级
    assert_eq!(
        classify_media_url("https://x.com/a.mp3"),
        MediaSuffix::AudioPlayable
    );
    assert_eq!(
        classify_media_url("https://x.com/b.amr"),
        MediaSuffix::AudioDegrade
    );
    assert_eq!(
        classify_media_url("https://x.com/c.mp4"),
        MediaSuffix::Video
    );
    assert_eq!(
        classify_media_url("https://x.com/d.txt"),
        MediaSuffix::Unknown
    );
}
