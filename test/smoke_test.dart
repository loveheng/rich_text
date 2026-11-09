import 'package:rich_text/markdown_subset.dart';
import 'package:test/test.dart';

void main() {
  final parser = const MarkdownSubsetParser();

  test('parse 委托 Rust 核心产出正确块树', () {
    final blocks = parser.parse('# Hello **world**\n\n- [ ] todo\n- [x] done\n\n> quote *i*');
    expect(blocks, isNotEmpty);
    expect(blocks.first, isA<HeadingBlock>());
    final h = blocks.first as HeadingBlock;
    expect(h.level, 1);
    expect(h.inline, hasLength(2)); // Text + Strong

    final list = blocks.whereType<ListBlock>().first;
    expect(list.ordered, isFalse);
    expect(list.items, hasLength(2));
    expect(list.items[0].done, isFalse);
    expect(list.items[1].done, isTrue);

    final quote = blocks.whereType<QuoteBlock>().first;
    expect(quote.children, isNotEmpty);
  });

  test('R3 引用链接仍由 Dart 预处理解析', () {
    final blocks = parser.parse('[text][id]\n\n[id]: https://e.x/z');
    final para = blocks.whereType<ParagraphBlock>().first;
    // 应出现 InlineLink（label=text, url=https://e.x/z）
    final hasLink = para.inline.any((n) => n is InlineLink);
    expect(hasLink, isTrue);
  });

  test('parseInline 仍走 Dart（goodshare 渲染兼容）', () {
    final inline = parser.parseInline('a **b** c');
    expect(inline, hasLength(3));
    expect(inline[1], isA<InlineStrong>());
  });
}
