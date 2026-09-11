# Todo

Working list for the live-inline editor. Check items off in this file. Order matches [ROADMAP.md](ROADMAP.md).

## Now — one caret, one layout

Types so the compiler can catch a wrong space:

- [x] Newtypes (or equivalent wrappers) for buffer offset, buffer column, visual column
- [x] Decide grapheme vs char for `TextPoint` / motion; update the comment that says “or” (char offsets; grapheme motion on Next — done)

Selection and caret on that map:

- [x] Render selection (`anchor`/`head`) through `char_map` using `theme.bg_selection`; Shift+arrows stay visible
- [x] Mouse drag to select; click without drag collapses to a caret
- [x] `window.focus` on launch and on editor click
- [x] Overlay caret (does not insert a 2px child into the run row); blink
- [x] Scroll the viewport so the caret stays in view
- [x] Store Up/Down preferred column as a **visual** column; map through `char_map` on the target line

Layout is a stage, not a guess:

- [x] Shaped-text layout object: glyph x/width, visual index, buffer index — paint, click, and caret all use it
- [x] Drop guessed 5.0 / 8.5 / 12.5 widths and the hardcoded 72px click origin
- [x] Wrap lines on words via GPUI `shape_text` (measured height; hit-test/caret/selection follow wraps)
- [x] Tests: visual ↔ buffer for concealed heading; Up/Down from heading onto paragraph; selection uses the same map as the caret

## Next — document parse and a real edit log

Parse a document:

- [x] Track fence-open state (or event parse) so code-block body is not a paragraph
- [x] Single prefix grammar used by parser, decorator, Enter, Backspace, and task toggle
- [x] `---` / `***` / `___` use a horizontal-rule marker, not `CodeFence`
- [x] Paint strikethrough; parse `***` as bold-italic (or drop unused `BoldItalic`)
- [x] Heading `group_range` is the `### ` token, not the whole line
- [x] Unmatched `` ` `` does not abort the rest of the line

Edit log:

- [x] Coalesce sequential inserts in undo; replacing a selection is one undo entry
- [x] `replace_range` orders start/end so it cannot panic
- [x] Rename `EditAction.start_byte` to the unit we actually store
- [x] Document generation (or equivalent version) increments on each applied edit
- [x] Move / backspace / delete by grapheme (`unicode-segmentation`) if that is the chosen unit

## Then — commands, then a thin view

- [ ] Commands module: wrap marks, continue/exit list, toggle task, indent — each returns one `Edit`; tests without GPUI
- [ ] Enter on `* [ ]` / indented tasks continues a task item; empty item exits the list
- [ ] Toggle task matches every form the grammar accepts
- [ ] Task glyph click toggles without also moving the caret
- [ ] Link click vs caret placement is defined (edit source vs open)
- [ ] `VisualRun` is role + source range + text; font size/color come from theme at layout time
- [ ] One heading type-scale table in `Theme` (`line_height_base` used); delete the 1.85 copies in decorator / click / paint
- [ ] Conceal by role: heading/list/task/quote prefixes reveal for the line; inline markers reveal by `group_range`; drop global Token vs Line; keep `Raw`
- [ ] Toolbar or equivalent for Bold / Italic / Code
- [ ] Tab / Shift+Tab indent and outdent the line or selection
- [ ] `EditorView` only binds input and paints layout (no prefix re-detection)

## Later — app and incrementality

- [ ] Copy source: entire buffer as Markdown (clipboard of the rope)
- [ ] Copy / cut / paste of the selection (multiline)
- [ ] Read / Edit toggle (Read: no caret/typing, mixed conceal always on; Edit: live loop; `Raw` is Edit-only)
- [ ] Open / save; title shows name and dirty state
- [ ] Cache projected lines by (document generation, conceal policy, caret token group / active line); invalidate a range
- [ ] Use theme tokens for accents (no hardcoded indigo / sky / emerald)
- [ ] Status bar: human conceal/read-edit label, not `{:?}`
- [ ] Navbar that fits the 640px minimum width without emoji-chip overflow

## Later docs

- [ ] `ARCHITECTURE.md` once Buffer → Parse → Project → Layout → Commands → View is stable
