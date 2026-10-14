# 数学符号/公式解析 评估（rich_text 共享包）

- status: accepted（已实施，2026-10-14 落地）
- updated: 2026-10-14
- 关联：goodshare 渲染层（KaTeX 等消费端负责呈现，本包只建模）；本仓 `lib/rich_text.dart`(抽象) + `lib/markdown_subset.dart`(实现)
- 结论：**把数学当成第二个「预扫描提取器」，完全镜像现有 `_extractCode`**；零运行时依赖、零渲染、parse→serialize 往返幂等。

---

## 1. 范围判断

本包是**纯 Dart 3、零依赖、不渲染**。所以「解析数学公式」在本包内 = **识别定界符 + 把 LaTeX 源码建模成 AST 节点**，不引入 KaTeX / flutter_math。渲染由各消费端（goodshare/goodhub）负责。

- 行内：`$E=mc^2$` → `InlineMath(source: 'E=mc^2')`
- 块级：`$$…$$`（可跨多行）→ `MathBlock(source: '…')`

`source` 存**裸 LaTeX**（不含定界符），与 `InlineCode.code` 同口径，序列化原样回写。

## 2. 现状

`$$…$$` 此前被 `_hasUnsupportedGfm` 当成「不支持 GFM」降级成段落并给 R1 提示（`markdown_subset.dart` 旧 `_hasUnsupportedGfm` / `normalizeAiMarkdown` 提示）。本改动把它从「降级」升级为「一等公民」。

## 3. 架构方案

现状已有「提取器优先于正则」的良好地基：`_extractCode` 是线性双扫描、`_inlinePattern` 是事后合并，**code 内部标记不递归**。数学与 code 同构（叶子节点、内容原样、不抢分组号），故最干净的做法是**把数学做成第二个提取器**，镜像 `_extractCode`：

- **不动** `_inlinePattern` 分组号（1–13 全保留）→ `inlineSpansOf` 的 `m.group(N)` 分支零改动风险；
- 新增 `_extractMath(text)` 线性扫描 `$…$`，与 code 提取共用 event 合并逻辑；code 优先于 math，math 优先于普通正则；
- 天然避开 §3.6 已为 code 弃用的「灾难性回溯」正则方案。

块级 `$$` 走责任链新增的 `_MathBlockSyntax`（`canParse` = 行首 `$$`；`parse` 吃到闭合 `$$`，无闭合降级段落不丢字）。

## 4. 改动面

**`lib/rich_text.dart`（抽象）**
- `InlineMark` 加 `math`；
- 新增 `InlineMath` / `MathBlock`（各含 `final String source`）；
- `inlineToPlain` / `blockToPlain` 加 `=> source`；
- `serializeInline` 加 `=> '\$$source\$'`，`serializeBlock` 加 `=> '\$\$$source\$\$'`；
- **同步硬约束**：`inlineNodesOf.styled` 是穷尽 switch，加枚举成员必须补 `InlineMark.math => InlineMath(text)`，否则编译不过；
- `_escapeInlineRe` 与转义组加 `$`，让字面 `$` 走 `\$` 转义保往返。

**`lib/markdown_subset.dart`（实现）**
- `_InlineEvent` 加 `math` 字段（与 `code` 并列）；
- `_extractMath`（镜像 `_extractCode`），与 code 重叠规则：code 优先；
- `_parseInlineImpl`：math event → `InlineMath`（**不递归**解析内部 `*` `_`）；
- `inlineSpansOf`：加 math 分支，emit `plain = source` + `math` run（**必须**与 `inlineToPlain` 逐字符一致，单测锁死）；
- 责任链加 `_MathBlockSyntax`（Divider 之后、Paragraph 之前）；`_isBlockStart` 纳入 `$$` 作块边界，防被段落续行吞掉；
- `_hasUnsupportedGfm` 删 `$$` 检测，HTML 标签检测保留；`normalizeAiMarkdown` 提示文案移除数学式；
- `lostMediaUrls` / `scanTodoTexts` 的 switch 有 `_ =>` 兜底，无需改。

**`test/rich_text_test.dart`**：行内数学解析 / `$$` 块跨行 / serialize 往返幂等 / 货币 `$` 与转义 `\$` 不误判 / `inlineSpansOf` 与 `inlineToPlain` 一致性。

## 5. 风险与取舍

### 5.1 `$` 货币歧义（唯一棘手项）
`售价 $5`、`` `$5 off` `` 等会被 `$…$` 误判。解法 = **边缘规则 + `\$` 转义**（已选，覆盖广、几乎零误伤）：
- 开口 `$` 后不能紧跟空格；闭合 `$` 前不能紧跟空格；
- 开口 `$` 前若为数字（如 `5$`）、闭合 `$` 后为数字（如 `$5`）则不视为数学；
- 行内数学不跨行（`\n` 即停）；`\$` 与 `$$` 跳过（`$$` 由块级专管）；
- 字面 `$` 用 `\$` 书写，序列化自动转义保往返。

### 5.2 还原优先级
code > math > 普通标记（`_inlinePattern`）。数学内部标记不递归，符合「内容原样建模、渲染交给消费端」。

### 5.3 降级铁律
`$$` 块无闭合 → 降级 `ParagraphBlock`，`$$` 字面保留，绝不丢字。

## 6. 验收

- `dart analyze`：无告警；
- `dart test`：全绿（含 6 个数学相关新增用例：行内解析、块级跨行、serialize 往返幂等、货币/`\$` 不误判、`inlineSpansOf` 与 `inlineToPlain` 逐字符一致）。
