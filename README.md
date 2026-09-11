# Mord

A **Word-like live Markdown editor** built in Rust with GPUI.

You edit real Markdown. The screen looks like a formatted document. Syntax markers (`#`, `**`, `- [ ]`) stay hidden until the caret needs them. There is no split source/preview pane.

## What this is

- **Source of truth:** a Markdown rope (`DocumentBuffer`)
- **View:** a visual projection (styled runs, concealed markers, Word-like page)
- **Hard problem:** keep caret, click, and edits correct while markers appear and disappear
- **Reveal modes:** token (caret on that mark), line (whole active line), raw (always show syntax)

Mord is early. The live conceal loop is the product; file chrome and a full CommonMark suite come later.

## Run

```bash
cargo run
```

```bash
cargo test
```

Requires a GPU-capable environment (GPUI).

## Repo

| Path | Role |
| --- | --- |
| `src/main.rs` | GPUI app window |
| `src/editor/` | Buffer, selection, parser, decorator (visual projection) |
| `src/ui/` | Theme and `EditorView` (paint + input) |

## Docs

- [AGENTS.md](AGENTS.md) — spec and constraints for humans and agents
- [ROADMAP.md](ROADMAP.md) — milestones
- [TODO.md](TODO.md) — current work list

`ARCHITECTURE.md` will be added once the engine (parse → decorate → map → paint) is stable enough to document as a contract.
