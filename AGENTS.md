# Mord — agent notes

Mord is a single-pane **Word-like live Markdown editor**. The buffer is Markdown. The screen is a formatted projection of that buffer. Users edit in place.

## Objective

1. WYSIWYG of the source, not a preview pane. Headings, emphasis, lists, tasks, quotes, and links look like a document.
2. Conceal/reveal is the product. Markers stay hidden until the caret is on that token (`TokenReveal`), on that line (`LineReveal`), or the user wants raw syntax (`Raw`).
3. Visual text and buffer text are two coordinate systems. Caret, click, Backspace, and Up/Down must map through the same `char_map` the painter uses.
4. Word-like editing still mutates Markdown (list continuation, task toggles, heading scale). There is no parallel rich-text document.
5. Stay fast: GPUI + rope, incremental work later — not “reparse the world” forever.

## Non-goals (until the engine is honest)

- Split source / HTML preview
- A second document model (HTML, Scribe, ProseMirror-style)
- Full CommonMark / GFM coverage for its own sake
- App chrome (open/save, find, settings) as the main effort

File I/O and clipboard matter for a finished app. They are not the thesis.

## Invariants

- **Markdown in the rope is truth.** Visual runs are derived. Never invent a second store of “formatted text.”
- **Paint and hit-test must agree.** If a glyph is at visual column V, click and caret at V must map to the same buffer column.
- **Preferred column for Up/Down is visual**, then mapped to the target line — not a raw buffer column (concealed `### ` would jump).
- **Conceal modes change visibility, not the buffer.**
- **Token reveal is per-marker `group_range`**, not “any caret on the line” (headings currently violate this).
- **Fenced code is a document span**, not a one-line prefix. Body lines must not run inline conceal.

## Layout

- `src/editor/buffer.rs` — rope, undo (char offsets, not bytes)
- `src/editor/selection.rs` — caret + selection
- `src/editor/parser.rs` — line/document parse → spans with `char_range` + `group_range`
- `src/editor/decorator.rs` — spans + conceal + caret → visual runs + `char_map`
- `src/ui/editor_view.rs` — input, commands, GPUI paint (too large; split when touching it)
- `src/ui/theme.rs` — colors and type scale; paint must use it

Prefix grammar (list, task, heading, quote) must live in **one** place. Parser, decorator, Enter, Backspace, and task toggle must not each re-detect syntax.

## Parser

A custom parser exists because we need **per-character ranges and token groups** for live reveal. `pulldown-cmark` may feed events into that model. Do not replace the editor with HTML-in-a-webview or a static CommonMark render.

## When you change mapping or conceal

Add or extend tests for:

- Inactive heading: visual `My Title` ↔ buffer after `### `
- Token reveal: only the mark under the caret unhides
- Click past the visual end → line end, not a panic
- Up/Down from a concealed heading onto a paragraph keeps visual column
- Selection is visible and uses the same map as the caret

Do not “fix” click accuracy with more guessed glyph widths. Use GPUI shaped-text hit testing (or equivalent layout metrics).

## Commands

```bash
cargo test
cargo run
```

Prefer small, testable editor changes over expanding `EditorView::render`.
