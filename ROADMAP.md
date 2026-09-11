# Roadmap

The product is the **live inline loop**. Six stages; two are missing.

```
Buffer → Parse → Project → Layout → Commands → View
         (doc)   (conceal)  (shape)   (edits)    (GPUI)
```

Today: a rope, a line parser, a decorator rebuilt every frame, and `EditorView` doing layout, commands, and paint. Chrome waits until caret, click, and conceal share one layout.

## Now — one caret, one layout

Make paint and hit-test the same object. That is engine honesty *and* the layout stage.

- Newtypes for the spaces we already mix: buffer offset, buffer column, visual column (pixels stay in layout)
- Commit `TextPoint` to graphemes or chars; stop saying “or”
- Selection painted through `char_map`; mouse drag; Shift+arrows match the screen
- Focus on launch and click
- Overlay caret (not a 2px sibling in the run row); blink; scroll to caret
- Up/Down preferred column is **visual**, mapped per line
- One shaped-text layout: paint, click, and caret all read it (no guessed glyph widths)
- Wrap on words via `shape_text` (currently `shape_line` — no style-run flex wrap; soft-wrap still open)

Do not “fix” clicks by tuning `8.5` px advances.

## Next — document parse and a real edit log

The parser must see a document. Undo must be something parse/project can invalidate against.

- Blocks, not line prefixes: fence + body + close; no inline conceal inside code
- One prefix grammar (heading / quote / list / task) used by parse, project, Enter, Backspace, toggle
- Horizontal rule is not `CodeFence`; heading `group_range` is the `### ` token
- Strikethrough painted; `***` defined or `BoldItalic` removed
- Unmatched `` ` `` does not abort the rest of the line
- Coalesced undo; replace-selection is one entry; `replace_range` cannot take an inverted range
- `EditAction.start_byte` becomes a char/grapheme offset; document generation increments on edit
- Motion / delete by grapheme if that is the unit we committed to

`pulldown-cmark` may feed events into spans. It must not become HTML in a webview.

## Then — commands, then a thin view

Word-like behavior is Markdown in, Markdown out — testable without GPUI.

- Command layer: wrap marks, continue/exit lists, toggle task, indent — each returns one `Edit`
- List/task/quote continuation matches the grammar (`* [ ]`, indent included)
- Task click does not also move the caret; link click vs edit is explicit
- `VisualRun` carries role (marker / emphasis / code / link / task), not font size; theme owns the type scale (one table, not 1.85 copied three times)
- Discoverable Bold / Italic / Code; Tab / Shift+Tab indent
- `EditorView` binds input and paints layout; it does not re-detect prefixes

## Later — app and incrementality

After the six stages exist.

- Clipboard (multiline)
- Open / save; title; dirty flag
- Cache projected lines by (document generation, conceal mode, caret token group); dirty a range
- Theme tokens for accents; human status labels; navbar that fits 640px

## Docs

- `ARCHITECTURE.md` — when Buffer → Parse → Project → Layout → Commands → View is a contract (offset types, cache keys, invalidation). Not before.
