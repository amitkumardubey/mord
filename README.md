# Mord

A **live Markdown editor** with Word-like *editing* (in-place format, not a split preview), built in Rust with GPUI.

You edit real Markdown. Live hides closed marks. Raw is the source. Look is a later pass.

## What this is

- **Source of truth:** a GitHub Flavored Markdown rope (`DocumentBuffer`)
- **Live:** formatted projection (styled runs, concealed closed marks); look is polish
- **Raw:** source with light syntax coloring
- **Hard problem:** caret, click, and edits stay correct while Live hides markup

Mord is early. The live loop is the product. The source bar is **GitHub Flavored Markdown** (not a private dialect). File chrome comes with that, not instead of it.

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

`ARCHITECTURE.md` will be added once Buffer → Parse → Project → Layout → Commands → View is stable enough to document as a contract. See [ROADMAP.md](ROADMAP.md).
