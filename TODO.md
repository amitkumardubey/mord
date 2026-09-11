# Todo

Working list for the live-inline editor. Check items off in this file. Order matches [ROADMAP.md](ROADMAP.md).

## Engine honesty

- [ ] Render selection (`anchor`/`head`) using `theme.bg_selection`; Shift+arrows stay visible
- [ ] Mouse drag to select; click without drag collapses to a caret
- [ ] `window.focus` on launch and on editor click
- [ ] Overlay caret (does not insert a 2px child into the run row); blink
- [ ] Scroll the viewport so the caret stays in view
- [ ] Store Up/Down preferred column as a **visual** column; map through `char_map` on the target line
- [ ] Hit-test clicks with GPUI text layout (drop guessed 5.0 / 8.5 / 12.5 widths and hardcoded 72px origin)
- [ ] Wrap lines on words, not `flex_wrap` at style-run edges
- [ ] Tests: visual ↔ buffer for concealed heading; click past visual end; Up/Down from heading onto paragraph

## Document Markdown

- [ ] Track fence-open state (or event parse) so code-block body is not a paragraph
- [ ] Single prefix grammar used by parser, decorator, Enter, Backspace, and task toggle
- [ ] `---` / `***` / `___` use a horizontal-rule marker, not `CodeFence`
- [ ] Paint strikethrough; parse `***` as bold-italic (or drop unused `BoldItalic`)
- [ ] Heading `group_range` is the `### ` token, not the whole line
- [ ] Unmatched `` ` `` does not abort the rest of the line
- [ ] Coalesce sequential inserts in undo; replacing a selection is one undo entry
- [ ] `replace_range` orders `start`/`end` so it cannot panic
- [ ] Move / backspace / delete by grapheme (`unicode-segmentation`)

## Word-like editing

- [ ] Enter on `* [ ]` / indented tasks continues a task item; empty item exits the list
- [ ] Toggle task matches every form the parser accepts
- [ ] Task glyph click toggles without also moving the caret (stop the line handler)
- [ ] Link click vs caret placement is defined (edit source vs open)
- [ ] Toolbar or equivalent for Bold / Italic / Code
- [ ] Tab / Shift+Tab indent and outdent the line or selection

## App (after the loop works)

- [ ] Copy / cut / paste
- [ ] Open / save; title shows name and dirty state
- [ ] Cache decorated lines; invalidate a range instead of the whole buffer
- [ ] Split document + commands out of `EditorView::render`
- [ ] Use theme tokens for accents (no hardcoded indigo / sky / emerald)
- [ ] Status bar: human conceal-mode label, not `{:?}`
- [ ] Navbar that fits the 640px minimum width without emoji-chip overflow

## Later docs

- [ ] `ARCHITECTURE.md` once parse → decorate → map → paint is stable
