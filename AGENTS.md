# AGENTS.md

A terminal Markdown previewer: pure Rust, no Node / browser / server. The crate is published as `mdview-rs` (the name `mdview` was taken on crates.io); the library and the binary are both `mdview`. Rust edition 2024, `rust-version = 1.85`.

## Commands

```sh
cargo fmt --all --check                          # CI gates on this — run it before committing
cargo clippy --all-targets -- -D warnings        # same gate: a warning is an error
cargo test --all-targets                         # unit + snapshot + logic tests, no terminal needed
INSTA_UPDATE=always cargo test --test render_snapshot   # re-baseline after a render change
cargo run -- examples/demo.md                    # look at it locally
cargo build --release                            # binary at target/release/mdview
```

`.cargo/config.toml` is gitignored (a dev-machine proxy; CI uses the official crates-io registry). Do not try to commit it, and do not assume another machine has a proxy.

## Architecture

One hard rule: **pure logic stays separate from terminal IO.** That split is the reason everything is testable — put new code where this rule says, not where it is convenient.

- Pure logic (zero terminal IO, unit-testable directly): `src/render.rs`, `src/wrap.rs`, `src/search.rs`, `src/mermaid.rs`, `src/app.rs` (state machine + `handle(events)`: keys in, state out)
- Touches the screen / keyboard: `src/view.rs` (single draw entry point `draw(f, app)`), `src/term.rs` (`TermGuard` for raw mode / alternate screen / mouse, plus the panic hook), `src/event.rs` (`collect` aggregates events, `TICK = 250ms`), `src/main.rs` (the loop)
- Rendering has exactly one entry point: `mdview::to_doc(src, width, opts)` (`preview` in `lib.rs` is only an alias). A width change re-renders; a height change only clamps the scroll offset. Do not add a second render path.

## Rendering conventions (read before editing — easy to break)

- Headings do not print their `#`, and code blocks are not wrapped in ``` fences: `src/theme.rs` switches both off by returning empty strings from `heading_marker` / `code_block_fence`. Heading level comes from colour and weight only.
- The outline maps rendered lines back to headings by comparing the line's style against `heading(level)` plus a text prefix (`is_heading_line` / `source_headings` in `render.rs`), which is why a `#` comment inside a code block never becomes a heading. Change a style and you must change this too.
- Wrapping is by display width (`unicode-width` / `unicode-segmentation`) so CJK text cannot skew tables and code frames. That is why `wrap.rs` exists.
- mermaid renders as box-drawing text, never as an image protocol; a diagram that fails to parse is pasted back as a code block with a status-bar warning instead of blanking the screen.
- Known upstream warts in `mermaid-text` (waiting on upstream — do not patch around them locally): after a `gitGraph` merge the commit dot is off by one cell, and `«interface»` in `classDiagram` can overlap a border by one cell.

## Tests

- Shared constructors live in `tests/support/mod.rs`: `render` / `render_no_mermaid` / `plain` / `theme` / `opts` / `args`. The theme is hard-coded dark with mermaid off so snapshots do not drift with CI's `TERM` / `NO_COLOR` — new tests go through these helpers, never `Theme::detect`.
- `temp_md()` gives each test case its own temp directory: parallel cases must not share one `t.md` (it exists so the live-reload tests can write files).
- Snapshots are in `tests/snapshots/` (insta). When render behaviour changes, re-run with `INSTA_UPDATE=always` and review the diff — do not blind-refresh snapshots to make a test pass.

## Language

- Code comments and internal docs: Chinese, one line stating the why is enough.
- User-facing strings (`src/cli.rs` help, the status bar, `HELP` in `view.rs`) and `README.md`: English. Chinese docs live separately in `docs/README.zh-CN.md` — when you change one, change the other.

## Releasing

Three steps: bump `version` in `Cargo.toml` → commit → tag `v<version>` and push. The tag triggers CI: `check` (fmt/clippy/test) → `build` (3 platforms) → `release` (GitHub Release with the tarballs) → `publish` (crates.io, using the repo secret `CARGO_REGISTRY_TOKEN`).

CI fails if the tag version does not equal the `Cargo.toml` version. `cargo publish` runs without `--no-verify` on purpose: it unpacks and rebuilds the tarball, which is a local dry-run.
