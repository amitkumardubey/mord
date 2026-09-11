# Mord — agent notes

Mord is a single-pane **live Markdown editor**. The buffer is Markdown. The screen is a formatted projection. Users edit in place.

**Word-like means behavior, not look.** In-place formatting, sticky/select styles, list continuation, toolbar + shortcuts — not Calibri, a print page, or Office chrome. Visual density (thin vs thick, type scale, chrome) is a **later polish pass** and can still change as long as paint goes through `Theme`.

## Objective

1. WYSIWYG of the source, not a preview pane. Headings, emphasis, lists, tasks, quotes, links, images, and tables are visible as formatting — modestly, not as a Word skin.
2. Live hides closed GFM markup; incomplete syntax may show while typing. `Raw` is source (light syntax color, 1:1 with the rope). Popovers (link, image, …) edit attributes without leaking markup.
3. Visual text and buffer text are two coordinate systems. Caret, click, Backspace, and Up/Down must map through the same `char_map` the painter uses.
4. Editing still mutates Markdown (list continuation, task toggles, wrap/sticky, heading commands). There is no parallel rich-text document.
5. Stay fast: GPUI + rope, incremental work later — not “reparse the world” forever.

## Non-goals

- Split source / HTML preview
- A second document model (HTML, Scribe, ProseMirror-style)
- Replacing the editor with `pulldown-cmark` HTML in a webview
- GitHub-only social extras (mentions, issue refs) unless we explicitly add them
- App chrome (find, settings) as the main effort until GFM in Live is honest
- Opening or importing `.docx`, HTML, RTF, or other rich files. **Open/save is Markdown** (`.md`). Plain `.txt` may open as a Markdown buffer (no format conversion). Export PDF/docx is one-way from the rope.

File I/O and clipboard matter for a finished app. They are not a substitute for parse/layout.

## Markdown: GFM is the bar

The buffer must be **GitHub Flavored Markdown** (CommonMark + GFM extras: tables, task lists, strikethrough, autolinks, GFM tagfilter). Round-trip is the rope, not a rendered HTML string.

`pulldown-cmark` (already a dependency) may drive **block/inline events with source offsets** into our span model (`char_range` + `group_range`). It must not become the screen.

Passing every CommonMark spec example in Live is the long goal; until a construct has ranges + hit-test, it may be plain in Live and correct in Raw. Do not silently drop GFM syntax from the buffer.

## Product agreements

Locked in with the user; agents follow these over older Token/Line wording.

**Modes**
- Global Token vs Line is gone. **Live** and **Raw** only.
- **Live:** formatted Markdown in place, closed markup hidden. Task/list glyphs, not source `- [ ]`. Heading hashes hidden; change level with a control or Raw. How large headings are, and how much margin the buffer has, is polish — not an invariant.
- **Raw:** source editor (VS Code/Cursor-like). Visual column = buffer column. Light syntax coloring only — no heading type scale, no `•`/`☑` substitutes.
- **Read / Edit:** Read is Live with no caret/typing; Edit is the live loop. Raw is Edit-only.

**Formatting (always writes Markdown)**
- Type markers by hand (power user).
- **Select then format:** wrap/unwrap Bold / Italic / Code like Word (per line if the selection is multiline; emphasis does not span a blank line).
- **Sticky typing:** toolbar/shortcut with a collapsed caret arms Bold or Italic for *new characters on this line* until Enter, toggle off, or the caret moves away. Does not restyle existing text after the caret. Do not insert empty `****`. Code is selection-wrap only, not sticky. Headings / lists / quotes / indent stay line commands.
- Inside an existing bold (or italic) span, typing continues that style; clicking Bold turns it *off* for what comes next (split the span).
- **Shortcuts:** same commands as the toolbar (Ctrl on Windows/Linux, Cmd on macOS). No Vim mode; do not bind unmodified typing keys.
  - **B** / **I** — Bold / Italic (wrap or sticky)
  - **E** — inline code (selection only; keep E, not backtick)
  - **K** — link popover (when it exists)
  - **1**…**6** — heading level
  - **Z** / **Shift+Z** or **Y** — undo / redo; **A** select all; **S** save
  - **C** / **X** / **V** — copy / cut / paste selection; **Shift+C** copy source
  - **Tab** / **Shift+Tab** — indent / outdent
  - **Shift+X** — strikethrough (when the command exists)
  - **Shift+R** — Live / Raw (Edit only)
  - **Shift+E** — Read / Edit (when that toggle exists)
  - Enter / Backspace / arrows stay as now
  - A shortcut list in Later chrome; toolbar stays the discoverable path.

**Chrome**
- Link and image **popovers** edit the rope without showing markup in Live. Heading level control belongs with that.
- One **icon set** (open-source or custom) for the toolbar; no emoji as UI.
- **Open/save:** `.md` only, plus optional `.txt` as a Markdown buffer (no conversion). No `.docx` / HTML / RTF import.
- **Export** PDF and `.docx` is one-way from the rope.
- **Copy source** (whole rope) then selection clipboard.
- **Images:** `![alt](src)` in the rope; Live paints; files on disk after save exists. No blobs in the rope.
- **Charts:** not GFM. Optional later: image, or one fenced language (e.g. Mermaid) Live-rendered. No chart widget / webview / Excel object.
- **Theming:** two built-in appearances (light, dark). Every color and type-scale value comes from `Theme` (including Raw syntax colors, code/task accents, chrome). No hardcoded hex in paint or navbar. Custom user themes / marketplace are Later. PDF export uses a print/light layout, not “screenshot of dark Live.” Follow OS light/dark when we have settings.
- **Polish:** after the engine is honest. Look is **not locked**. Retune type, spacing, chrome, heading scale via `Theme` without retouching parse/commands. A thin (not Zed, not Office) direction is the current preference, not a freeze.

**Markdown**
- Bar is **GFM** (CommonMark + tables, tasks, strikethrough, autolinks, tagfilter). Mentions are out unless we add them on purpose.
- `pulldown-cmark` may supply events with source offsets into our spans. Never HTML-in-a-webview.
- Do not drop GFM from the buffer. Live may show a construct as plain until it has layout.

## Invariants

- **Markdown in the rope is truth.** Visual runs are derived. Never invent a second store of “formatted text.”
- **Paint and hit-test must agree.** If a glyph is at visual column V, click and caret at V must map to the same buffer column.
- **Preferred column for Up/Down is visual**, then mapped to the target line — not a raw buffer column (concealed `### ` would jump).
- **Conceal changes visibility, not the buffer.**
- **Live hides closed marks.** Do not use a global Token vs Line switch. Incomplete syntax may reveal while composing. **Raw** is 1:1 source with light color.
- **Fenced code is a document span**, not a one-line prefix. Body lines must not run inline conceal.

## Layout

- `src/editor/buffer.rs` — rope, undo (char offsets, not bytes)
- `src/editor/offset.rs` — `BufferOffset` / `BufferCol` / `VisualCol` newtypes
- `src/editor/selection.rs` — caret + selection; Up/Down preferred column is visual
- `src/editor/parser.rs` — line/document parse → spans with `char_range` + `group_range`
- `src/editor/prefix.rs` — one prefix grammar (heading / quote / list / task)
- `src/editor/commands.rs` — Markdown edits as one `Edit` each (wrap, Enter, Backspace, toggle, indent)
- `src/editor/decorator.rs` — spans + conceal + caret → visual runs + `char_map`
- `src/editor/layout.rs` — visual runs → GPUI `TextRun`s; byte ↔ visual mapping
- `src/ui/document_line.rs` — shaped-line paint, overlay caret, selection quads
- `src/ui/editor_view.rs` — input, commands, chrome (keep paint logic in `document_line`)
- `src/ui/theme.rs` — colors and type scale; paint must use it

Prefix grammar (list, task, heading, quote) must live in **one** place. Parser, decorator, Enter, Backspace, and task toggle must not each re-detect syntax.

## Parser

A custom span model exists because we need **per-character ranges and token groups** for Live mapping and commands. GFM compliance comes from mapping a real CommonMark/GFM parse (e.g. `pulldown-cmark`) onto that model — not from growing ad-hoc `starts_with` forever, and not from painting HTML.

## When you change mapping or conceal

Add or extend tests for:

- Inactive heading: visual `My Title` ↔ buffer after `### `
- Live heading: hashes stay hidden while the caret is on the title; inline `**` on the same line stays hidden unless the wrap is still incomplete
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
