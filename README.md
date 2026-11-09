# rich_text · 原生 SDK（纯 Rust）

单一 Rust 核心 + UniFFI 绑定的 Markdown 子集解析/序列化 SDK（方案 B）。AST 跨 FFI 边界走
**JSON 序列化形态**，纯文本/标量函数直接返回（零拷贝）。坐标契约：行内 span 为 UTF-8 字节偏移。

> 设计 SSOT、行为基准仍在 goodshare 仓 `docs/design/`。本分支 `android-native-sdk` 是 Rust
> 重实现 + UniFFI 绑定，由黄金语料 `native/rich_text_core/tests/` 锁死一致。详见
> [`docs/eval/native-rewrite-eval.md`](docs/eval/native-rewrite-eval.md)。

## 布局

```
native/
  rich_text_core/     # 唯一事实源：纯 Rust 核心，零运行时依赖（fancy_regex + serde）
  rich_text_uniffi/   # UniFFI 绑定层（proc-macro 模式）
    src/lib.rs        # #[uniffi::export] 薄函数 + #[derive(uniffi::Record)] 结果类型
    bindings/         # uniffi-bindgen 生成的 Kotlin / Swift 桩
    scripts/gen-bindings.sh
docs/                 # 评估类文档（含方案 B 落地记录）
```

## 构建

```bash
# 需要 Rust stable 工具链
cd native/rich_text_core && cargo test        # 6 单测 + 5 黄金语料
cd native/rich_text_uniffi && cargo build     # 产出 target/debug/librich_text_uniffi.so
```

## 导出 API（JSON 串 / 标量 进出）

| 函数 | 说明 |
|---|---|
| `rt_version()` | SDK 版本 |
| `parse_to_json(md) -> json` | Markdown 子集 → 块树 JSON |
| `serialize_blocks_from_json(json) -> md` | 块树 JSON → Markdown（编辑器回写唯一出口） |
| `rt_markdown_to_plain(md)` | 整篇 → 纯文本 |
| `rt_title_to_plain(src)` | 标题 → 纯文本 |
| `rt_inline_spans_of(src) -> json` | 行内 → 纯文本 + 字节坐标样式 run |
| `rt_classify_media_url(url)` | 媒体分类：`audioPlayable`/`audioDegrade`/`video`/`unknown` |
| `rt_normalize_ai_markdown(md) -> {markdown, notes}` | AI 写入归一 |
| `rt_scan_todo_texts(md) -> [str]` | 待办项纯文本列表 |
| `rt_lost_media_urls(orig, incoming) -> [str]` | AI 回写护城河：丢失的媒体 url |

各端消费与绑定生成见 [`native/README.md`](native/README.md)。
