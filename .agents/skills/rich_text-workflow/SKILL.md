---
name: rich_text-workflow
description: rich_text（纯 Dart 3.13 Markdown 子集解析共享包，零运行时依赖、无 Flutter）工作流事实源：模块结构、包名、构建/测试命令、Dart 环境硬约束；goodshare 与 goodhub 共依赖。
---

# rich_text · 工作流事实源

> 写法/架构规范见各自代码与项目索引 skill；本文件只放「现场实测」的硬事实，防双源漂移。
> 设计 rationale SSOT 在 goodshare 仓库（其 docs 下的 design 目录；本包不携设计文档）。

## 包与依赖

- 包名：`rich_text`（`pubspec.yaml` `name`）；`version: 0.1.0`；`publish_to: 'none'`（不发布，path/git 依赖）。
- 运行时依赖：**零**（README 明示 zero dependency, no Flutter）。
- dev_dependencies：`flutter_lints ^5.0.0`、`test ^1.25.0`；environment `sdk: ^3.13.4`。
- 消费方：goodshare（交互渲染）、goodhub（只读宿主渲染）通过 `path: ../rich_text` 或 git 依赖同一份。

## 模块结构

- `lib/rich_text.dart`：**抽象层** —— AST 模型（`RichBlock` sealed / `InlineNode` sealed 及其子类）、`RichTextParser` 接口、纯 AST 运算（`serializeInline`/`serializeBlock`/`serializeBlocks` 往返幂等、`inlineToPlain`/`blockToPlain`、`classifyMediaUrl`、`adjustRuns`/`toggleInlineMarkRuns`/`inlineNodesOf` 编辑原语）。含媒体后缀白名单枚举 `MediaSuffix`。
- `lib/markdown_subset.dart`：**实现层** —— `MarkdownSubsetParser`（implements `RichTextParser`）+ 块语法责任链（`_LineReader` + `_BlockSyntax` 注册表）+ 依赖 parser 的下游 helper：`normalizeAiMarkdown`/`lostMediaUrls`/`scanTodoTexts`/`markdownToPlain`/`inlineSpansOf`/`inlineToPlainText`/`titleToPlain`。本文件 `import` 并 `export` 抽象库，消费方按需一条 import 得全量。

## 构建 / 测试命令（均已本环境实测跑通）

> 环境前提：本机 Dart 由 Flutter SDK 捆绑，**未入 PATH**，需先 `export PATH="$HOME/flutter/bin:$PATH"`（本机 `dart` 路径 `/home/zzh/flutter/bin/dart`，版本 3.13.4）。

- `dart pub get` → `Got dependencies!`
- `dart analyze` → `Analyzing rich_text... No issues found!`
- `dart test` → `All tests passed!`（`test/rich_text_test.dart`，4 用例：块识别 / 降级不吞 / todo / 行内）

> AI 写入归一 `normalizeAiMarkdown` 与「降级铁律」等关键口径仅在本包冒烟覆盖；全量语义测试在 goodshare 仓。

## 环境硬约束

- Dart SDK `^3.13.4`（>=3.13 特性，勿降 SDK 下限）。
- 禁增运行时依赖（共享包契约：纯 Dart、零依赖、无 Flutter）。
- 渲染器刻意不进本包（各端渲染器不同），本包只产出 AST + 序列化 + 编辑原语。
- 改动须保持 `parse`↔`serialize` 往返幂等（既有单测锁定），任何 parser 改动需跑 `dart test`。
