---
name: rich_text-index
description: rich_text（纯 Dart Markdown 子集解析共享包）的「功能 → 代码落点 + 文档落点」归属索引。修改、新增、排查、评审解析/序列化/编辑原语或 AI 归一前，先本表定位，避免全仓扫描。
---

# rich_text 功能索引

机制/格式/维护协议见全局 skill `project-index`。

## 用法

1. 按归属表（或业务别名）定位领域。
2. 以代码落点路径为搜索锚点展开（如 `grep -rn <关键词> lib/`）；展开结果只用于当前任务，禁止写回本表。
3. 定位不到才全量 grep / find。

## 归属表

| 领域 | 覆盖功能 | 代码落点（锚点 · 展开） | 文档落点 |
|---|---|---|---|
| 解析实现 | `MarkdownSubsetParser`、`_LineReader`、块责任链、行内解析 `_parseInlineImpl`/`_extractCode`、`serialize*` 往返 | `lib/markdown_subset.dart`（`grep -rn <k> lib/markdown_subset.dart`） | — |
| AST 模型/接口 | `RichBlock`/`InlineNode` 及其子类、`RichTextParser` 接口、媒体白名单 `MediaSuffix` | `lib/rich_text.dart`（`grep -rn <k> lib/rich_text.dart`） | — |
| 编辑原语 | `adjustRuns`/`toggleInlineMarkRuns`/`inlineNodesOf`/`InlineRun`/`SpanListRun` | `lib/rich_text.dart`（编辑原语段） | — |
| AI 归一 | `normalizeAiMarkdown`（脚注/子集外降级）、`lostMediaUrls` 护城河 | `lib/markdown_subset.dart`（normalizeAiMarkdown） | — |
| 媒体分类 | `classifyMediaUrl`/后缀白名单 | `lib/rich_text.dart`（classifyMediaUrl） | — |
| 纯文本/标题 | `markdownToPlain`/`titleToPlain`/`inlineToPlainText` | `lib/markdown_subset.dart` | — |
| 测试 | 解析/降级/行内冒烟（全量语义在 goodshare 仓） | `test/rich_text_test.dart` | — |
| 增量解析评估 | 编辑器增量/懒解析优化评估（proposed，未实施） | `docs/incremental-parsing-eval.md` | `docs/incremental-parsing-eval.md` |

跨域隐式契约：
- 联动: 解析实现（序列化 `serialize*` 与 `parse` 必须互斥往返幂等——任何 parser 改动同步改测试）
- 联动: AI 归一（落点降级不吞内容铁律，与解析实现共用降级语义）

## 业务别名映射

- **富文本 / markdown 子集 / 块树 / 行内节点**：→ 解析实现、AST 模型
- **降级 / 不吞内容 / 脚注映射**：→ AI 归一
- **样式化编辑 / run / 选区套格式**：→ 编辑原语
- **增量 / 懒解析 / 流式**：→ 增量解析评估

## 维护约定

- 可推导的不维护（路径/命名靠 grep 现场 derive），不可推导的才登记；类清单/文件触点不进表。
- 新增领域/大功能加一行；禁止铺满。
- 索引随功能 PR 一起改，不走单独文档流程。
