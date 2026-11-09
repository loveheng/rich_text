# rich_text · docs 结构索引

> 纯结构索引：域 → 文档 → 一句话定位。详细功能归属见项目索引 skill（`rich_text-index`）与 `rich_text-docs` skill。
> 文档规范（frontmatter 时效 / 落点命名 / Tombstone / 写后自检）见全局 `docs-spec` §1–§7。

## eval（评估）

- [编辑器增量/懒解析 优化评估](incremental-parsing-eval.md) — status: proposed；更新 2026-10-14；结论：先方案 A（尾块增量）后 B（块级 memoize），C（逐字符真增量）不做。
- [数学符号/公式解析 评估](eval/math-parsing-eval.md) — status: accepted（已实施）；更新 2026-10-14；结论：数学作为第二个预扫描提取器，完全镜像 `_extractCode`，零依赖零渲染、往返幂等。
- [平台原生实现改造 评估](eval/native-rewrite-eval.md) — status: accepted（已拍板走方案 B）；更新 2026-11-09；结论：单一原生核心（Rust）+ UniFFI 各端绑定，AST 跨 FFI 走 JSON 序列化形态；方案 C 否决并随纯 Rust 化从仓库移除。
  - 落地位置：本分支 `android-native-sdk` 的 `native/rich_text_core`（事实源）+ `native/rich_text_uniffi`（绑定层），黄金语料见 `native/rich_text_core/tests/`。
