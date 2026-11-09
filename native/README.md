# rich_text 原生 SDK（方案 B：单一 Rust 核心 + UniFFI 绑定）

纯 Rust 核心 [`rich_text_core`](rich_text_core/) 是唯一事实源，经 UniFFI 绑定层
[`rich_text_uniffi`](rich_text_uniffi/) 暴露给 Kotlin / Swift / Dart。AST 跨 FFI 边界走
**JSON 序列化形态**，纯文本/标量函数直接返回（零拷贝）。坐标契约：行内 span 为 UTF-8 字节偏移。

> 设计 SSOT、行为基准仍在 goodshare 仓 `docs/design/` 与 Dart `lib/`。本分支 `android-native-sdk`
> 是 Rust 重实现 + UniFFI 绑定，由黄金语料 `rich_text_core/tests/` 锁死一致。详见
> `docs/eval/native-rewrite-eval.md`。

## 目录

```
native/
  rich_text_core/     # 事实源：纯 Rust 核心，零运行时依赖（fancy_regex + serde）
  rich_text_uniffi/   # UniFFI 绑定层（proc-macro 模式）
    src/lib.rs        # #[uniffi::export] 薄函数 + #[derive(uniffi::Record)] 结果类型
    bindings/         # 由 uniffi-bindgen 生成的 Kotlin / Swift 桩（见下文）
    scripts/gen-bindings.sh
```

## 构建

```bash
# 需要 Rust 工具链（stable）
cd rich_text_uniffi
cargo build                 # 产出 target/debug/librich_text_uniffi.so（cdylib）
cargo build --release       # 发布用
```

核心单测 / 黄金语料：

```bash
cd rich_text_core
cargo test                  # 6 单测 + 5 黄金语料（往返幂等 + 结构不变量）
```

## 导出 API（JSON 串 / 标量 进出）

| 函数 | 说明 |
|---|---|
| `rt_version()` | SDK 版本 |
| `parse_to_json(md) -> json` | Markdown 子集 → 块树 JSON（各端反序列化为本地模型） |
| `serialize_blocks_from_json(json) -> md` | 块树 JSON → Markdown（编辑器回写唯一出口） |
| `rt_markdown_to_plain(md)` | 整篇 → 纯文本（预览/检索） |
| `rt_title_to_plain(src)` | 标题 → 纯文本 |
| `rt_inline_spans_of(src) -> json` | 行内 → 纯文本 + 字节坐标样式 run（`{plain, runs}`） |
| `rt_classify_media_url(url)` | 媒体分类：`audioPlayable`/`audioDegrade`/`video`/`unknown` |
| `rt_normalize_ai_markdown(md) -> {markdown, notes}` | AI 写入归一 |
| `rt_scan_todo_texts(md) -> [str]` | 待办项纯文本列表 |
| `rt_lost_media_urls(orig, incoming) -> [str]` | AI 回写护城河：丢失的媒体 url |

## Kotlin / Android 消费

1. 生成桩：`uniffi-bindgen generate --library target/debug/librich_text_uniffi.so --language kotlin --out-dir bindings/kotlin`（或 `./scripts/gen-bindings.sh`）。
2. 把 `bindings/kotlin/**` 与编译出的各架构 `.so`（见下）纳入 Android 模块 `jniLibs/`。
3. 运行时（JNA）：`System.loadLibrary("rich_text_uniffi")` 后直接调用 `RichTextUniffi.rtVersion()` 等。

多架构 `.so` 需以 NDK 交叉编译：`cargo install cargo-ndk` 后
`cargo ndk --platform 21 --targets arm64-v8a,armeabi-v7a,x86_64,x86 -p rich_text_uniffi build --release`，
产物落到 `jniLibs/` 对应 ABI 子目录。

## Swift / iOS 消费

1. 生成桩：`uniffi-bindgen generate --library target/debug/librich_text_uniffi.so --language swift --out-dir bindings/swift`。
2. 把 `.swift` 与 `librich_text_uniffi.dylib`/`.a` 接入 Xcode；或用 Swift Package Manager 包一层。

## Dart 绑定

核心 `uniffi-bindgen` **不**产出 Dart。改用社区 `uniffi-dart-bindgen`（基于 `dart:ffi` + `ffigen`）：
以 `librich_text_uniffi.so` 与 UDL/metadata 生成 Dart FFI 绑定。本仓暂不内置 Dart 桩，待 CLI 接入后再补。

## 坐标契约

`rt_inline_spans_of` 返回的 `runs[].start/end` 是 **UTF-8 字节偏移**（与 Rust 核心一致）。各端按本地
字符串编码换算：Kotlin/Java `String` 为 UTF-16 code unit，Swift `String` 为 Unicode scalar——
在边界将字节偏移映射为本地索引后再套选区/样式，避免错位。
