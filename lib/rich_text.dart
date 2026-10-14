/// 富文本模型与解析（Markdown 子集 → 块树）。
///
/// SSOT：docs/design/content-pipeline.md §3 / §5，docs/design/ui-spec.md §2.2。
///
/// 为什么要自建而不用 `flutter_markdown_plus`：
/// 1. **待办勾选在渲染器架构下无法实现**——渲染器输出不可交互文本流，而
///    `ui-spec` §7 要求 `human_md` 中 `[ ]` 可勾选、状态写 `todo_state_json`
/// 2. 该包是原包归档停更后的社区分叉
/// 3. 文档感全靠排版参数，受渲染器内部结构限制
///
/// 降级铁律：**宁可样式平，不可吞内容**——不认识的块一律降级为段落，
/// 绝不丢弃任何字符。
///
/// ---
/// **共享包注记（2026-10-14 抽取）**：本库原为 goodshare 自写 Markdown 子集解析器
/// （goodshare `lib/doc/rich_text.dart`），2026-10-14 抽为独立纯 Dart 共享包
/// `rich_text`，由 goodshare 与 goodhub 共同 path/git 依赖，消除双仓副本分叉
/// （D12 曾记「第二处复用再评估抽包」，此处即抽包点）。设计 rationale 的 SSOT
/// 文档（content-pipeline / rich-text-media / rich-text-gfm /
/// rich-text-component / ui-spec）在 goodshare 仓 docs/design/。渲染器不进本包
/// ——各端渲染器刻意不同（goodhub 只读宿主渲染 / goodshare 交互渲染）。
///
/// ---
/// **库分层（2026-10-14 DIP 拆分）**：本 library = **抽象**（AST 模型 + [RichTextParser]
/// 接口 + 纯 AST 运算 `serialize*` / `inlineToPlain` / `blockToPlain` /
/// `classifyMediaUrl` / 编辑原语）。具体 `MarkdownSubsetParser` + 依赖 parser 的下游
/// helper 在 `markdown_subset.dart`（实现库，import 并 export 本抽象库）。只消费 AST
/// 的渲染方仅依赖本抽象库，不牵入具体解析实现。

library;

// ---------- 行内 ----------

/// 行内节点。
sealed class InlineNode {
  const InlineNode();
}

/// 纯文本。
class InlineText extends InlineNode {
  const InlineText(this.text);

  final String text;
}

/// 粗体（`**` 或 `__`）。
class InlineStrong extends InlineNode {
  const InlineStrong(this.children);

  final List<InlineNode> children;
}

/// 斜体（`*` 或 `_`）。
class InlineEm extends InlineNode {
  const InlineEm(this.children);

  final List<InlineNode> children;
}

/// 下划线（`<u>` 标签）。
///
/// Markdown 标准无下划线语法——`<u>` 是 md 生态事实标准（HTML 内嵌），桌面端
/// 多数渲染器兼容；解析/渲染/序列化三出口同文件同源（block-format-input.md §2）。
class InlineUnderline extends InlineNode {
  const InlineUnderline(this.children);

  final List<InlineNode> children;
}

/// 行内代码。
class InlineCode extends InlineNode {
  const InlineCode(this.code);

  final String code;
}

/// 链接（`[文字](url)`）。**默认不可点击**（不引 `url_launcher`），
/// 仅以链接样式呈现。
///
/// [autolink] = true 表示由自动链接（裸 `url` / `www.` / `mailto:`，见 §3.6 ⑤）
/// 识别而来：label 即 url，序列化回裸 url（不包 `<>` / 不包 `[]()`）。
class InlineLink extends InlineNode {
  const InlineLink({
    required this.label,
    required this.url,
    this.autolink = false,
  });

  final String label;
  final String url;
  final bool autolink;
}

/// 删除线（~~text~~，GFM 官方扩展，§3.6 收编）。
class InlineStrikethrough extends InlineNode {
  const InlineStrikethrough(this.children);

  final List<InlineNode> children;
}

/// 高亮（==text==，Pandoc/Obsidian 事实标准，§3.6 收编）。
class InlineHighlight extends InlineNode {
  const InlineHighlight(this.children);

  final List<InlineNode> children;
}

/// 行内数学（`$...$`，LaTeX 源码原样存储，内部标记不递归解析；渲染由消费端负责）。
///
/// [source] = 定界符内裸 LaTeX（不含 `$`），与 [InlineCode] 同口径；序列化回
/// `$source$`、行内纯文本取 [source]（[inlineToPlain] / [inlineSpansOf] 严格镜像，
/// 单测锁死）。行内数学是「提取器」叶子节点（镜像 `_extractCode`），与 `_inlinePattern`
/// 的递归标记互不重叠。
class InlineMath extends InlineNode {
  const InlineMath(this.source);

  final String source;
}

// ---------- 块级 ----------

/// 块节点。
sealed class RichBlock {
  const RichBlock();
}

/// 标题（level 1-6）。
class HeadingBlock extends RichBlock {
  const HeadingBlock({required this.level, required this.inline});

  final int level;
  final List<InlineNode> inline;
}

/// 段落。
class ParagraphBlock extends RichBlock {
  const ParagraphBlock(this.inline);

  final List<InlineNode> inline;
}

/// 表格列对齐（GFM `:---` / `:--:` / `---:`）。
enum TableAlign { left, center, right }

/// 表格块（GFM 官方扩展，§3.6 ④）。单元格以**纯文本字面**存储（避免单元格内
/// `|` 转义复杂度，列为后续项），渲染层对单元格做行内解析（§3.5 / §5.4
/// 「表格内行内格式仍解析」），故往返幂等且 UI 无残壳。
class TableBlock extends RichBlock {
  const TableBlock({required this.header, required this.rows, this.align});

  final List<String> header;
  final List<List<String>> rows;
  final List<TableAlign>? align;
}

/// 引用块（内部递归解析，支持引用内多段）。
class QuoteBlock extends RichBlock {
  const QuoteBlock(this.children);

  final List<RichBlock> children;
}

/// 列表项。`done` 非 null 表示这是待办项（可勾选）。
///
/// [lines] 首行为 `- `/`1. `/`- [ ] ` 标记后的内容；后续元素为「续行」
/// （markdown 中以 ≥2 空格缩进表示，解析层归入同一项）。单行项仅一个元素。
/// [inline] 为 [lines] 首行的兼容 getter，单行调用可继续用。
class ListItem {
  const ListItem(this.lines, {this.done});

  final List<List<InlineNode>> lines;

  /// 首行内容（兼容旧单行调用；多行项请直接用 [lines]）。
  List<InlineNode> get inline =>
      lines.isEmpty ? const [InlineText('')] : lines.first;

  /// true = 已完成 `[x]`，false = 未完成 `[ ]`，null = 普通列表项。
  final bool? done;
}

/// 列表（有序 / 无序）。
class ListBlock extends RichBlock {
  const ListBlock({required this.ordered, required this.items});

  final bool ordered;
  final List<ListItem> items;
}

/// 代码块（围栏 ```）。
class CodeBlock extends RichBlock {
  const CodeBlock({required this.code, this.language});

  final String code;
  final String? language;
}

/// 分隔线。
class DividerBlock extends RichBlock {
  const DividerBlock();
}

/// 行内图片块（整行 `![alt](url)`，SSOT：docs/design/rich-text-media.md §2）。
class ImageBlock extends RichBlock {
  const ImageBlock({required this.url, this.alt = ''});

  final String url;

  /// 图片说明（用户/AI 原话，规则层禁加类型前缀——往返幂等）。
  final String alt;
}

/// 行内音频块（整行 `[label](url)` 且 url 后缀命中音频白名单）。
class AudioBlock extends RichBlock {
  const AudioBlock({required this.url, this.label = ''});

  final String url;
  final String label;
}

/// 行内视频块（整行 `[label](url)` 且 url 后缀命中视频白名单）。
class VideoBlock extends RichBlock {
  const VideoBlock({required this.url, this.label = ''});

  final String url;
  final String label;
}

/// 块级数学公式（`$$...$$`，可跨多行；LaTeX 源码原样存储，渲染由消费端负责）。
///
/// [source] = 开闭 `$$` 之间内容（可能含换行），序列化回 `$$source$$`；无闭合时
/// 解析层降级为段落（[降级铁律]：不丢内容）。
class MathBlock extends RichBlock {
  const MathBlock(this.source);

  final String source;
}

// ---------- 媒体 url 后缀分类 ----------

/// 媒体 url 后缀归类（rich-text-media.md §2 白名单两档）。
enum MediaSuffix {
  /// 音频，可直接内嵌播放。
  audioPlayable,

  /// 音频但平台兼容性存疑（如 .amr）：parse 照常归 AudioBlock（AST 不携带
  /// 能力信息），呈现层降级为文件卡不进播放器。
  audioDegrade,

  /// 视频。
  video,

  /// 非媒体或未识别。
  unknown,
}

const Set<String> _audioPlayableExt = {'.mp3', '.m4a', '.aac', '.wav', '.opus'};
const Set<String> _audioDegradeExt = {'.amr'};
const Set<String> _videoExt = {'.mp4', '.mov', '.webm', '.m3u8'};

/// url → 后缀归类。取 `Uri.parse(url).path`（天然剥离 query 与 fragment）、
/// toLowerCase 后与白名单比对；parse 与呈现层共用此单一事实源。
MediaSuffix classifyMediaUrl(String url) {
  final path = Uri.tryParse(url)?.path ?? url;
  final dot = path.lastIndexOf('.');
  if (dot < 0 || dot == path.length - 1) return MediaSuffix.unknown;
  final ext = path.substring(dot).toLowerCase();
  if (_audioPlayableExt.contains(ext)) return MediaSuffix.audioPlayable;
  if (_audioDegradeExt.contains(ext)) return MediaSuffix.audioDegrade;
  if (_videoExt.contains(ext)) return MediaSuffix.video;
  return MediaSuffix.unknown;
}

/// AI 写入归一结果（rich-text-gfm.md §2 层2 语义映射 + R1 note 载体）。
///
/// [markdown] = 归一后（子集内、无残壳）的 md；[notes] = 本次发生的降级映射
/// 说明，供命令返回体/MCP 响应带 R1 note 回传 AI 可感知（映射即降级须告知）。
class AiNormalizeResult {
  const AiNormalizeResult(this.markdown, this.notes);

  final String markdown;
  final List<String> notes;
}

/// 行内节点 → 纯文本（待办回调取文本、复制、检索预览用）。
///
/// 只取「人读到的字」，不含标记符号。
String inlineToPlain(List<InlineNode> nodes) => nodes
    .map(
      (n) => switch (n) {
        InlineText(:final text) => text,
        InlineStrong(:final children) => inlineToPlain(children),
        InlineEm(:final children) => inlineToPlain(children),
        InlineUnderline(:final children) => inlineToPlain(children),
        InlineCode(:final code) => code,
        InlineLink(:final label) => label,
        InlineStrikethrough(:final children) => inlineToPlain(children),
        InlineHighlight(:final children) => inlineToPlain(children),
        InlineMath(:final source) => source,
      },
    )
    .join();

/// 块 → 纯文本（预览 / 检索用）。
String blockToPlain(RichBlock block) => switch (block) {
  HeadingBlock(:final inline) => inlineToPlain(inline),
  ParagraphBlock(:final inline) => inlineToPlain(inline),
  QuoteBlock(:final children) => children.map(blockToPlain).join('\n'),
  ListBlock(:final items) =>
    items.map((i) => i.lines.map(inlineToPlain).join('\n')).join('\n'),
  CodeBlock(:final code) => code,
  DividerBlock() => '',
  // 媒体块降级（三出口之 blockToPlain，rich-text-media.md §2）
  ImageBlock(:final alt) => alt.isEmpty ? '[图片]' : '[图片: $alt]',
  AudioBlock(:final label) => label.isEmpty ? '[音频]' : '[音频: $label]',
  VideoBlock(:final label) => label.isEmpty ? '[视频]' : '[视频: $label]',
  MathBlock(:final source) => source,
  TableBlock(:final header, :final rows) => [
    ...header,
    for (final r in rows) ...r,
  ].join(' '),
};

// ---------- 行内样式 run（block-format-input.md §4 样式化编辑层地基） ----------

/// 行内样式种类（编辑态样式化层用；渲染即 switch 出对应 TextStyle 增量）。
enum InlineMark {
  bold,
  italic,
  code,
  underline,
  link,
  strikethrough,
  highlight,
  math,
}

/// 一个样式 run：**纯文本坐标** [start, end)（标记字符已剥除）。
/// 嵌套（如粗体含于下划线）以重叠 run 表达——渲染侧按字符合成样式。
class InlineRun {
  const InlineRun(this.start, this.end, this.mark, {this.url});

  final int start;
  final int end;
  final InlineMark mark;

  /// 仅 link run 携带：plain 只含 label，url 由 run 载荷保全（数据无损防线）。
  final String? url;
}

/// [inlineSpansOf] 的返回：纯文本 + 纯文本坐标的样式 run 列表。
class InlineSpans {
  const InlineSpans({required this.plain, required this.runs});

  final String plain;
  final List<InlineRun> runs;
}

/// 行级档位 run（速记所见即所得的行标题，2026-10-03）：**纯文本坐标**
/// [start, end)，恒与实际整行对齐（宿主侧 snap；详情页标题是块属性不用它）。
class SpanLevelRun {
  const SpanLevelRun(this.start, this.end, this.level);

  final int start;
  final int end;

  /// 0 正文 / 1 一级标题 / 2 二级标题（0 不参与渲染，序列化侧消费）。
  final int level;
}

/// 列表种类（速记所见即所得的「行列表」维度，2026-10-06 拍板：粘贴/导入 md
/// 列表 → 无标记列表/待办）：bullet = 无序圆点（`- ` / `* `），
/// todo = 待办（`- [ ]` / `- [x]`），ordered = 有序项（`N. ` / `N) `，序号
/// **不由 run 存储**——始终由所处连续有序组实时推算，保证增删项后自动重排，
/// 与详情页解析器「每个 ListBlock 从 1 重排」同口径）。
/// 与行级标题同源——**行属性**，整行表达，恒与实际整行对齐（宿主侧 snap）。
enum SpanListKind { bullet, todo, ordered }

/// 行列表 run：**纯文本坐标** [start, end)，覆盖一整行（不含行尾 `\n`）；
/// 多行列表项时 [end] 会吞入后续「≥2 空格缩进」的续行作几何延伸——仍只算
/// **一个**列表项（凹槽只按 [start] 所在首行绘制一次，续行留白不重复出标记）。
/// 标题与列表互斥（一行只认一种前缀，标题优先）。
class SpanListRun {
  const SpanListRun(this.start, this.end, this.kind, {this.done = false});

  final int start;
  final int end;
  final SpanListKind kind;

  /// 待办勾选态（仅 [SpanListKind.todo] 有意义）；渲染侧消费，不改正文。
  final bool done;
}

/// 文本变更区间（旧坐标）：最长公共前/后缀夹出「删旧 + 插新」窗口。
/// 纯函数，供 [adjustRuns] 与会话层随动使用。
(int, int) spanChangeRange(String oldText, String newText) {
  var p = 0;
  while (p < oldText.length && p < newText.length && oldText[p] == newText[p]) {
    p++;
  }
  var sOld = oldText.length;
  var sNew = newText.length;
  while (sOld > p && sNew > p && oldText[sOld - 1] == newText[sNew - 1]) {
    sOld--;
    sNew--;
  }
  return (p, sOld);
}

/// 文字变更时 run 随动（纯函数）：删除区裁剪、其后位移、run 内插入延展。
/// 重叠模型下不做合并——渲染按字符合成，语义即正确。
List<InlineRun> adjustRuns(
  List<InlineRun> runs,
  int start,
  int oldEnd,
  int insertLen,
) {
  final delta = insertLen - (oldEnd - start);
  final out = <InlineRun>[];
  for (final r in runs) {
    var s = r.start;
    var e = r.end;
    if (e <= start) {
      // 完全在变更区前：不动（含「插入点=run 末尾」——边界不延续，
      // 新文字是否入格式由激活态决定，先选后打拍板）
    } else if (s >= oldEnd) {
      // 完全在变更区后（含「插入点=run 起点」）：整体位移
      s += delta;
      e += delta;
    } else if (start == oldEnd) {
      // 纯插入且严格位于 run 内部：延展
      if (s < start) e += insertLen;
    } else {
      // 删除/替换：横跨（或触及）替换窗的 run 涵盖插入段（字级替换不打断
      // 样式连续性），尾部在窗内的 run 裁掉窗内部分；完全在窗内的 run 删空。
      final ns = s < start ? s : start;
      final ne = e >= oldEnd
          ? start + insertLen + (e - oldEnd)
          : ns + (e - start > 0 ? 0 : 0) + (e - start).clamp(0, start - s);
      s = ns;
      e = ne;
      if (e <= s) continue; // run 被删空
    }
    out.add(InlineRun(s, e, r.mark, url: r.url));
  }
  return out;
}

/// 选区套格式（编辑态样式化层）：在 [start, end) 上切换 [mark]。
/// 重叠模型：先按边界切分跨 start/end 的 mark run，整段已覆盖则摘掉、否则清掉
/// 重叠部分后整段盖上——纯函数，供 [ComposerDoc.applyMarkToSelection] 复用。
List<InlineRun> toggleInlineMarkRuns(
  List<InlineRun> runs,
  InlineMark mark,
  int start,
  int end,
) {
  if (end <= start) return runs;
  // 1. 边界切分：把跨 start/end 的 mark run 断开，避免局部覆盖残留
  final split = <InlineRun>[];
  for (final r in runs) {
    if (r.mark != mark) {
      split.add(r);
      continue;
    }
    if (r.start < start && start < r.end) {
      split.add(InlineRun(r.start, start, mark, url: r.url));
      split.add(InlineRun(start, r.end, mark, url: r.url));
      continue;
    }
    if (r.start < end && end < r.end) {
      split.add(InlineRun(r.start, end, mark, url: r.url));
      split.add(InlineRun(end, r.end, mark, url: r.url));
      continue;
    }
    split.add(r);
  }
  // 2. 区间内 mark 覆盖度判定
  final intervals = <(int, int)>[];
  for (final r in split) {
    if (r.mark != mark) continue;
    final s = r.start.clamp(start, end);
    final e = r.end.clamp(start, end);
    if (e > s) intervals.add((s, e));
  }
  intervals.sort((a, b) => a.$1.compareTo(b.$1));
  var covered = start;
  for (final (s, e) in intervals) {
    if (s > covered) break;
    covered = e > covered ? e : covered;
  }
  final fullyCovered = covered >= end;
  // 3. 清掉区间内 mark run（无论开/关都先清，避免重叠残留）
  final cleared = split
      .where((r) => !(r.mark == mark && r.end > start && r.start < end))
      .toList();
  if (fullyCovered) return cleared;
  cleared.add(InlineRun(start, end, mark));
  return cleared;
}

/// runs + plain → 行内节点树（slice-4 序列化回写与块重建的唯一出口）。
/// 重叠 run 按字符合成标记集合，相邻同集合（含同 link url）段合并；
/// 嵌套优先级固定为枚举顺序（bold > italic > code > underline > link）。
/// link run 的 url 从载荷还原——编辑链接文字不丢 url（数据无损防线）。
List<InlineNode> inlineNodesOf(String plain, List<InlineRun> runs) {
  final sets = List<Set<InlineMark>>.generate(
    plain.length,
    (_) => {},
    growable: false,
  );
  final urls = List<String?>.generate(
    plain.length,
    (_) => null,
    growable: false,
  );
  for (final r in runs) {
    for (var i = r.start; i < r.end && i < plain.length; i++) {
      sets[i].add(r.mark);
      if (r.mark == InlineMark.link) urls[i] = r.url;
    }
  }
  InlineNode styled(String text, Set<InlineMark> marks, String? url) {
    InlineNode n = InlineText(text);
    for (final m in InlineMark.values) {
      if (!marks.contains(m)) continue;
      n = switch (m) {
        InlineMark.bold => InlineStrong([n]),
        InlineMark.italic => InlineEm([n]),
        InlineMark.code => InlineCode(text),
        InlineMark.underline => InlineUnderline([n]),
        InlineMark.link => InlineLink(label: text, url: url ?? ''),
        InlineMark.strikethrough => InlineStrikethrough([n]),
        InlineMark.highlight => InlineHighlight([n]),
        InlineMark.math => InlineMath(text),
      };
    }
    return n;
  }

  final out = <InlineNode>[];
  var i = 0;
  while (i < plain.length) {
    var j = i + 1;
    while (j < plain.length &&
        sets[j].length == sets[i].length &&
        sets[j].containsAll(sets[i]) &&
        urls[j] == urls[i]) {
      j++;
    }
    final seg = plain.substring(i, j);
    out.add(styled(seg, sets[i], urls[i]));
    i = j;
  }
  return out;
}

// ---------- 序列化 ----------/// 行内纯文本需转义的字符：`\` 本身与子集语法标记（`*` `_` `[` `` ` ``）。
final RegExp _escapeInlineRe = RegExp(r'[\\*_[`$]');

String _escapeText(String s) =>
    s.replaceAllMapped(_escapeInlineRe, (m) => '\\${m[0]}');

/// 行内节点 → Markdown 子集串。与 [MarkdownSubsetParser.parseInline] 互逆：
/// 纯文本中的语法标记一律转义，保证「字面内容」往返不变形。
String serializeInline(List<InlineNode> nodes) => nodes
    .map(
      (n) => switch (n) {
        InlineText(:final text) => _escapeText(text),
        InlineStrong(:final children) => '**${serializeInline(children)}**',
        InlineEm(:final children) => '*${serializeInline(children)}*',
        InlineUnderline(:final children) =>
          '<u>${serializeInline(children)}</u>',
        InlineCode(:final code) => '`$code`',
        InlineLink(:final label, :final url, :final autolink) =>
          autolink ? url : '[${_escapeText(label)}]($url)',
        InlineStrikethrough(:final children) =>
          '~~${serializeInline(children)}~~',
        InlineHighlight(:final children) => '==${serializeInline(children)}==',
        InlineMath(:final source) => '\$$source\$',
      },
    )
    .join();

/// 块 → Markdown 子集串。与 [MarkdownSubsetParser.parse] 互逆（三出口护栏
/// 之 serialize，SSOT：docs/design/rich-text-component.md §5）。
String serializeBlock(RichBlock block) => switch (block) {
  HeadingBlock(:final level, :final inline) =>
    '${'#' * level} ${serializeInline(inline)}',
  ParagraphBlock(:final inline) => serializeInline(inline),
  QuoteBlock(:final children) =>
    children
        .map(serializeBlock)
        .map((md) => md.split('\n').map((l) => '> $l').join('\n'))
        // 子块间补空引用行（`>`），否则重解析时相邻段落会并段
        .join('\n>\n'),
  ListBlock(:final ordered, :final items) =>
    items
        .asMap()
        .entries
        .map((e) {
          final lines = e.value.lines;
          final first = serializeInline(lines.first);
          final done = e.value.done;
          final head = done != null
              ? '- [${done ? 'x' : ' '}] $first'
              : (ordered ? '${e.key + 1}. $first' : '- $first');
          final cont = lines
              .skip(1)
              .map((l) => '  ${serializeInline(l)}')
              .join('\n');
          return cont.isEmpty ? head : '$head\n$cont';
        })
        .join('\n'),
  CodeBlock(:final code, :final language) => '```${language ?? ''}\n$code\n```',
  DividerBlock() => '---',
  // 媒体块出口标准 Markdown 链接语法，MCP 桌面端零感知（§2）；
  // label/alt 原话直出，禁注入类型前缀（往返幂等）
  ImageBlock(:final url, :final alt) => '![${_escapeText(alt)}]($url)',
  AudioBlock(:final url, :final label) => '[${_escapeText(label)}]($url)',
  VideoBlock(:final url, :final label) => '[${_escapeText(label)}]($url)',
  MathBlock(:final source) => '\$\$$source\$\$',
  TableBlock(:final header, :final rows, :final align) => _serializeTable(
    header,
    rows,
    align,
  ),
};

/// 表格 → Markdown 子集串（GFM 管线语法；无尾随换行，交由 [serializeBlocks] 块间空行）。
String _serializeTable(
  List<String> header,
  List<List<String>> rows,
  List<TableAlign>? align,
) {
  String row(List<String> cells) => '| ${cells.join(' | ')} |';
  final a = align ?? [for (final _ in header) TableAlign.left];
  final sep =
      '| ${a.map((al) => switch (al) {
        TableAlign.center => ':---:',
        TableAlign.right => '---:',
        TableAlign.left => ':---',
      }).join(' | ')} |';
  final buf = StringBuffer()
    ..writeln(row(header))
    ..writeln(sep);
  for (final r in rows) {
    buf.writeln(row(r));
  }
  return buf.toString().trimRight();
}

/// 块列表 → Markdown 子集串（块间空行分隔；编辑器回写唯一出口）。
String serializeBlocks(List<RichBlock> blocks) =>
    blocks.map(serializeBlock).join('\n\n');

// ---------- 解析 ----------

/// 富文本解析器接口。
abstract class RichTextParser {
  /// Markdown 子集 → 块树。永不抛异常，永不丢内容。
  List<RichBlock> parse(String markdown);

  /// 行内解析（供外部复用，如标题文案）。
  List<InlineNode> parseInline(String text);
}
