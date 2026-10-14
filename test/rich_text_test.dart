import 'package:rich_text/markdown_subset.dart';
import 'package:test/test.dart';

/// 解析器冒烟（共享包 rich_text）：钉块识别与「降级不吞内容」铁律等关键口径；
/// 全量语义测试在 goodshare 仓。
void main() {
  const parser = MarkdownSubsetParser();

  test('标题/段落/列表/代码块识别', () {
    final blocks = parser.parse(
        '# 大标题\n\n正文一段。\n\n- 甲\n- 乙\n\n```dart\nvoid main() {}\n```');
    expect(blocks, hasLength(4));
    expect(blocks[0], isA<HeadingBlock>());
    expect(blocks[1], isA<ParagraphBlock>());
    final list = blocks[2] as ListBlock;
    expect(list.ordered, isFalse);
    expect(list.items, hasLength(2));
    expect((blocks[3] as CodeBlock).language, 'dart');
    expect((blocks[3] as CodeBlock).code, contains('void main()'));
  });

  test('降级铁律：不认识的行不吞字符（降级为段落）', () {
    final raw = '普通段落\n怪标记 <<<??>>> 尾部';
    final blocks = parser.parse(raw);
    final text = [
      for (final b in blocks)
        if (b is ParagraphBlock)
          [for (final n in b.inline) if (n is InlineText) n.text].join(),
    ].join('\n');
    expect(text, contains('怪标记 <<<??>>> 尾部'));
  });

  test('todo 勾选解析为 done 标记（宿主只读呈现 ☐/☑）', () {
    final blocks = parser.parse('- [ ] 未完\n- [x] 已完');
    final list = blocks.single as ListBlock;
    expect(list.items[0].done, isFalse);
    expect(list.items[1].done, isTrue);
  });

  test('行内：粗体/行内代码/链接', () {
    final blocks = parser.parse('**加粗** 与 `code` 与 [链](https://x.y)');
    final para = blocks.single as ParagraphBlock;
    expect(para.inline.whereType<InlineStrong>(), isNotEmpty);
    expect(para.inline.whereType<InlineCode>().single.code, 'code');
    expect(para.inline.whereType<InlineLink>().single.url, 'https://x.y');
  });

  test(r'行内数学 $...$ 解析为 InlineMath（内部标记不递归）', () {
    final blocks = parser.parse(r'公式 $E=mc^2$ 重要');
    final para = blocks.single as ParagraphBlock;
    final math = para.inline.whereType<InlineMath>().single;
    expect(math.source, 'E=mc^2');
    // 行内纯文本取裸源码（无 $ 定界符）
    expect(inlineToPlain(para.inline), '公式 E=mc^2 重要');
  });

  test(r'块级数学 $$...$$（可跨行）解析为 MathBlock', () {
    final md = '\$\$\n\\int_0^1 x^2\\,dx\n\$\$';
    final blocks = parser.parse(md);
    final math = blocks.single as MathBlock;
    expect(math.source, '\n\\int_0^1 x^2\\,dx\n');
  });

  test('数学 serialize 往返幂等', () {
    final inline = r'公式 $E=mc^2$ 重要';
    expect(serializeBlocks(parser.parse(inline)), inline);
    final block = '\$\$\n\\int x\n\$\$';
    expect(serializeBlocks(parser.parse(block)), block);
  });

  test(r'货币 $ 与转义 \$ 不被误判为数学', () {
    final md = r'售价 \$5 与 5\$x 测试';
    final blocks = parser.parse(md);
    final all = [
      for (final b in blocks)
        if (b is ParagraphBlock) ...b.inline,
    ];
    expect(all.whereType<InlineMath>(), isEmpty);
    // 字面 $ 经转义保往返
    expect(serializeBlocks(blocks), contains(r'\$'));
  });

  test('inlineSpansOf 与 inlineToPlain 逐字符一致（数学场景）', () {
    final src = r'公式 $E=mc^2$ 与 `code`';
    final spans = inlineSpansOf(src);
    final plain = inlineToPlain(parser.parseInline(src));
    expect(spans.plain, plain);
    expect(spans.runs.where((r) => r.mark == InlineMark.math), isNotEmpty);
  });
}
