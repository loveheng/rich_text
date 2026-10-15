---
name: rich_text-docs
description: rich_text 的 docs/ 项目数据：域目录表、lint/收集脚本路径、本仓例外；通用规范机制见全局 docs-spec §1–§7，本文件严禁复制规范正文。
---

# rich_text · docs/ 项目数据

> 规范机制（frontmatter 时效 / 落点与切片命名 / Mermaid / Tombstone / 引用移动与索引 / 写后自检）见全局 `docs-spec` §1–§7——**本文件只写项目数据**。

## 域目录表

| 域 | 定位 | 文档 |
|---|---|---|
| eval（评估） | 架构/优化评估类文档（如增量解析评估） | `docs/incremental-parsing-eval.md` |

## lint / 收集脚本

- 代码侧：`dart analyze`（无独立 docs lint 脚本）。
- 文档侧：遵循 `docs-spec` §1–§7，暂无独立 lint 脚本；写后自检按 docs-spec §7 手工执行。

## 本仓例外

- 暂无与 `docs-spec` 的冲突：现有文档均含 `status`/`updated` frontmatter 头，符合头部规范。
- `docs/` 当前仅 `eval` 一域一个文件，未引入多域目录结构；新增域时按 `docs-spec` §5 建 `docs/README.md` 域头并同步本表与项目索引 skill 文档落点列。
