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
- Soft-wrap on words via `shape_text` (measured line height; selection/caret/hit-test follow wraps)

Do not “fix” clicks by tuning `8.5` px advances.

## Next — document parse and a real edit log

The parser must see a document. Undo must be something parse/project can invalidate against.

- Blocks, not line prefixes: fence + body + close; no inline conceal inside code
- One prefix grammar (heading / quote / list / task) used by parse, project, Enter, Backspace, toggle
- Horizontal rule is not `CodeFence`; heading `group_range` is the `### ` token
- Strikethrough painted; `***` defined or `BoldItalic` removed
- Unmatched `` ` `` does not abort the rest of the line
- Coalesced undo; replace-selection is one entry; `replace_range` cannot take an inverted range
- `EditAction.start` is a char offset; document generation increments on edit
- Motion / delete by grapheme; offsets remain char-based

`pulldown-cmark` feeds events into spans. It must not become HTML in a webview.

## Then — commands, then a thin view

Word-like behavior is Markdown in, Markdown out — testable without GPUI.

- Command layer: wrap marks, continue/exit lists, toggle task, indent — each returns one `Edit`
- List/task/quote continuation matches the grammar (`* [ ]`, indent included)
- Task click does not also move the caret; link click vs edit is explicit
- `VisualRun` carries role (marker / emphasis / code / link / task), not font size; theme owns the type scale (one table, not 1.85 copied three times)
- Conceal by role: structural prefixes (heading / list / task / quote) reveal for the **line**; inline markers reveal by **token `group_range`**. `ConcealMode::{Live, Raw}` — no global Token vs Line switch
- Discoverable Bold / Italic / Code; Tab / Shift+Tab indent
- `EditorView` binds input and paints layout; it does not re-detect prefixes

## Word-like Live (after Then)

Replace click-to-reveal with a document Live and a source Raw.

- Live: closed marks stay hidden; show incomplete marks only while composing
- Raw: source + light syntax color; identity mapping
- Sticky Bold/Italic for collapsed-caret typing on the current line until Enter / toggle / caret leaves; no empty `****`
- Select-then-format wrap/unwrap; shortcuts = toolbar (see AGENTS.md shortcut table)
- Power-user chords: B/I/E, headings 1–6, K link, Shift+R Live/Raw, Shift+C copy source; no Vim
- Heading control in Live (hashes stay hidden)
- Link / image popovers

## GFM — source in, document out

The bar is **GitHub Flavored Markdown**, not a private dialect.

Hard parts (do not hand-wave):

- CommonMark **emphasis** (left/right flanking) is not naive `**` scanning
- **Nested blocks** (quote in list in quote) vs today’s mostly line-prefix parse
- **Tables** need a 2D layout, cell carets, and Tab-between-cells — not a decorated line
- **Images** (`![alt](src)`) are blocks/inlines we paint, not more text runs only
- **Indented code** (4 spaces) fights Tab-as-indent; prefer fenced code in Live commands
- **HTML** in GFM: keep in the rope; Live can show a placeholder, Raw shows tags — do not run a browser
- Spec tests are hundreds of examples; Live can lag Raw on edge cases, the buffer must still be GFM

Order after Word-like Live/Raw/sticky: images + link popover, tables, then event-parse + emphasis/autolink/reference links. Charts (e.g. Mermaid in a fence) are optional and not GFM.

## Later — app and incrementality

After the six stages exist. Clipboard and chrome wait until conceal-by-role is honest, so copy and read mode are not papering over three global modes.

- Clipboard: **copy source** first (entire rope as Markdown — unambiguous); then copy / cut / paste of the selection
- Read / Edit toggle: Read is a document (caret hidden, typing off, Live conceal always on); Edit is the live loop. `Raw` is Edit-only (source, not a third product mode)
- Open / save **Markdown files** (`.md`); optional `.txt` as Markdown (no importer). Dirty title. Export PDF / `.docx` is one-way from the rope — not a second document type
- Cache projected lines by (document generation, Live vs Raw, caret); dirty a range
- Theme tokens for accents; human status labels; navbar that fits 640px
- Icons: one open set (or custom) for toolbar; no emoji as the design system
- Link / image popovers edit the rope without showing markup in Live
- Charts optional (image or one fence language); not required for GFM
- Theming: light + dark; all paint via `Theme` tokens (Raw syntax colors included). Custom themes Later. Export PDF does not copy the editor’s dark chrome
- Dedicated UI/theme polish pass after the engine is honest. Look can still change (Theme tokens). Word = editing behavior, not Microsoft’s visual design.

## Docs

- `ARCHITECTURE.md` — when Buffer → Parse → Project → Layout → Commands → View is a contract (offset types, cache keys, invalidation). Not before.
