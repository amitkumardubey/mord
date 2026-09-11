# Roadmap

The product is the **live inline loop**: parse Markdown → decorate (conceal) → map visual ↔ buffer → paint and edit. Chrome and extra syntax wait until that loop is trustworthy.

## Now — engine honesty

Make the current document feel like one caret in one text.

- Paint selection; mouse can drag a range; Shift+arrows match what you see
- Focus the view on launch and on click
- Overlay caret (no 2px sibling that shoves glyphs); blink; scroll to caret
- Up/Down preferred column in **visual** space, mapped per line
- Clicks use real text layout, not hardcoded advances
- Wrap on words, not on style-run boundaries

## Next — document Markdown

The parser must see a document, not only a line.

- Fenced code is a block: fence + body + closing fence; no inline conceal inside
- One prefix grammar for heading / quote / list / task (shared by parse, decorate, Enter, Backspace, toggle)
- Horizontal rule is not a code-fence marker
- Emit and paint strikethrough; define `***` (bold-italic) instead of dead `BoldItalic`
- Token reveal uses each marker’s `group_range` (headings included)

Undo coalescing, grapheme motion, and inverted `replace_range` belong here because they sit under every edit.

## Then — Word-like editing

Behavior on the projection, still writing Markdown.

- List / task / quote continuation and exit match the parser (including `* [ ]` and indent)
- Task checkbox does not also steal the caret
- Links: visible as links; click vs edit is explicit
- Discoverable Bold / Italic / Code (toolbar or status), still wrapping source markers
- Indent as a command (Tab / Shift+Tab), not always four spaces

## Later — app

After caret, click, and conceal agree.

- Clipboard (copy / cut / paste), including multiline
- Open / save, window title, dirty flag
- Incremental parse/decorate (do not rebuild every line every frame)
- Split `EditorView` into document/commands vs paint

## Docs

- `ARCHITECTURE.md` — when parse → decorate → map → paint is a contract we can freeze (modules, offset types, invalidation). Not before.
