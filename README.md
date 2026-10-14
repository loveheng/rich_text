# rich_text

Shared **pure-Dart** Markdown-subset parser (zero dependency, no Flutter).

- `MarkdownSubsetParser` → block tree (`RichBlock`) + inline nodes (`InlineNode`)
- serializers (`serializeInline` / `serializeBlock` / `serializeBlocks`, parse↔serialize 幂等)
- editing primitives (`inlineSpansOf` / `adjustRuns` / `toggleInlineMarkRuns` / `inlineNodesOf`)
- helpers (`markdownToPlain` / `titleToPlain` / `classifyMediaUrl` / `normalizeAiMarkdown` / `lostMediaUrls` / `scanTodoTexts`)

**Origin**: self-written by goodshare (`lib/doc/rich_text.dart`). Extracted 2026-10-14 into this
shared package so goodshare and goodhub depend on one source of truth (was a forked copy in each repo).

**Renderers are NOT in this package** — each app keeps its own (goodhub: read-only host renderer;
goodshare: interactive renderer). Both consume the shared AST produced here.

Design-rationale SSOT docs live in goodshare `docs/design/` (content-pipeline / rich-text-media /
rich-text-gfm / rich-text-component / ui-spec).

## Consume

```yaml
dependencies:
  rich_text:
    path: ../rich_text   # or git: <repo> once hosted
```

```dart
import 'package:rich_text/rich_text.dart';
final blocks = MarkdownSubsetParser().parse(md);
```
