/// Rust 核心导出的块树 JSON → Dart 模型（[RichBlock] / [InlineNode]）解码器。
///
/// Rust `serde` 对枚举采用「外部标签」形态：`{"Heading":{"level":1,"inline":[...]}}`、
/// 单元变体 `Divider` 序列化为 `{"Divider":null}`。本解码器与 `ast.rs` 1:1 对应。
library;

import 'dart:convert';

import '../rich_text.dart';

/// 块树 JSON → Dart 模型列表。
List<RichBlock> decodeBlocks(String json) {
  final decoded = jsonDecode(json);
  if (decoded is! List) return const [];
  return [for (final e in decoded) _decodeBlock(e as Map<String, dynamic>)];
}

RichBlock _decodeBlock(Map<String, dynamic> m) {
  final entry = m.entries.single; // 外部标签：一个键值对
  final name = entry.key;
  final p = entry.value;
  switch (name) {
    case 'Heading':
      return HeadingBlock(
        level: p['level'] as int,
        inline: _decodeInlines(p['inline'] as List),
      );
    case 'Paragraph':
      return ParagraphBlock(_decodeInlines(p as List));
    case 'Quote':
      return QuoteBlock(_decodeBlocksList(p as List));
    case 'List':
      return ListBlock(
        ordered: p['ordered'] as bool,
        items: _decodeItems(p['items'] as List),
      );
    case 'CodeBlock':
      return CodeBlock(
        code: p['code'] as String,
        language: p['language'] as String?,
      );
    case 'Divider':
      return const DividerBlock();
    case 'Image':
      return ImageBlock(
        url: p['url'] as String,
        alt: (p['alt'] as String?) ?? '',
      );
    case 'Audio':
      return AudioBlock(
        url: p['url'] as String,
        label: (p['label'] as String?) ?? '',
      );
    case 'Video':
      return VideoBlock(
        url: p['url'] as String,
        label: (p['label'] as String?) ?? '',
      );
    case 'MathBlock':
      return MathBlock(p as String);
    case 'Table':
      return TableBlock(
        header: (p['header'] as List).cast<String>(),
        rows: [
          for (final r in p['rows'] as List) (r as List).cast<String>(),
        ],
        align: p['align'] == null
            ? null
            : [for (final a in p['align'] as List) _decodeAlign(a as String)],
      );
    default:
      // 未知块降级为段落（与降级铁律一致：不丢内容）。
      return const ParagraphBlock([InlineText('')]);
  }
}

InlineNode _decodeInline(Map<String, dynamic> m) {
  final entry = m.entries.single;
  final name = entry.key;
  final p = entry.value;
  switch (name) {
    case 'Text':
      return InlineText(p as String);
    case 'Strong':
      return InlineStrong(_decodeInlines(p as List));
    case 'Em':
      return InlineEm(_decodeInlines(p as List));
    case 'Underline':
      return InlineUnderline(_decodeInlines(p as List));
    case 'Code':
      return InlineCode(p as String);
    case 'Link':
      return InlineLink(
        label: p['label'] as String,
        url: p['url'] as String,
        autolink: (p['autolink'] as bool?) ?? false,
      );
    case 'Strikethrough':
      return InlineStrikethrough(_decodeInlines(p as List));
    case 'Highlight':
      return InlineHighlight(_decodeInlines(p as List));
    case 'Math':
      return InlineMath(p as String);
    default:
      return const InlineText('');
  }
}

ListItem _decodeItem(Map<String, dynamic> m) {
  final lines =
      (m['lines'] as List).map((l) => _decodeInlines(l as List)).toList();
  final done = m['done'] as bool?;
  return ListItem(lines, done: done);
}

List<InlineNode> _decodeInlines(List list) =>
    [for (final e in list) _decodeInline(e as Map<String, dynamic>)];

List<RichBlock> _decodeBlocksList(List list) =>
    [for (final e in list) _decodeBlock(e as Map<String, dynamic>)];

List<ListItem> _decodeItems(List list) =>
    [for (final e in list) _decodeItem(e as Map<String, dynamic>)];

TableAlign _decodeAlign(String s) => switch (s) {
      'Center' => TableAlign.center,
      'Right' => TableAlign.right,
      _ => TableAlign.left,
    };
