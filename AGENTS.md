# AGENTS.md

A terminal Markdown previewer: pure Rust, no Node / browser / server. The crate is published as `mdview-rs` (the name `mdview` was taken on crates.io); the library and the binary are both `mdview`. Rust edition 2024, `rust-version = 1.85`.

## Commands

```sh
cargo fmt --all --check                          # CI gates on this; CI only runs on tags, so this is the pre-tag gate
cargo clippy --all-targets -- -D warnings        # same gate: a warning is an error
cargo test --all-targets                         # unit + snapshot + logic tests, no terminal needed
INSTA_UPDATE=always cargo test --test render_snapshot   # re-baseline after a render change
cargo run -- examples/demo.md                    # look at it locally
cargo build --release                            # binary at target/release/mdview
```

`.cargo/config.toml` is gitignored (a dev-machine proxy; CI uses the official crates-io registry). Do not try to commit it, and do not assume another machine has a proxy.

## Architecture

One hard rule: **pure logic stays separate from terminal IO.** That split is the reason everything is testable — put new code where this rule says, not where it is convenient.

- Pure logic (zero terminal IO, unit-testable directly): `src/render.rs`, `src/wrap.rs`, `src/search.rs`, `src/mermaid.rs`, `src/files.rs` (directory listing + natural order), `src/config.rs` (the six TOML keys + the CLI > file > builtin merge), `src/app.rs` (state machine + `handle(events)`: keys in, state out)
- Touches the screen / keyboard: `src/view.rs` (single draw entry point `draw(f, app)`), `src/term.rs` (`TermGuard` for raw mode / alternate screen / mouse, plus the panic hook), `src/event.rs` (`collect` aggregates events, `TICK = 250ms`), `src/main.rs` (the loop)
- Rendering has exactly one entry point: `mdview::to_doc(src, width, opts)` (`preview` in `lib.rs` is only an alias). A width change re-renders; a height change only clamps the scroll offset. Do not add a second render path.

## Rendering conventions (read before editing — easy to break)

- Headings do not print their `#`, and code blocks are not wrapped in ``` fences: `src/theme.rs` switches both off by returning empty strings from `heading_marker` / `code_block_fence`. Heading level comes from colour and weight only.
- The outline maps rendered lines back to headings by comparing the line's style against `heading(level)` plus a text prefix (`is_heading_line` / `source_headings` in `render.rs`), which is why a `#` comment inside a code block never becomes a heading. Change a style and you must change this too.
- Wrapping is by display width (`unicode-width` / `unicode-segmentation`) so CJK text cannot skew tables and code frames. That is why `wrap.rs` exists.
- mermaid renders as box-drawing text, never as an image protocol; a diagram that fails to parse is pasted back as a code block with a status-bar warning instead of blanking the screen.
- Search keeps two paths on purpose: `search::compile` returns `Pattern::Lit` unless the query
  contains a regex metacharacter, so a plain query behaves and costs exactly what it did before
  regex support. A trailing `\c` / `\C` is a vim-style case flag stripped there — if you touch that
  code, keep the escape-parity check (`\\c` is a literal backslash + c, not a flag).
- Search indexes `RenderedDoc.logical` (one entry per *unwrapped* rendered line), not `plain`.
  tui-markdown already joins soft-wrapped paragraphs into one line and `wrap_line` hard-wraps it
  afterwards, so that list is what makes a query match across a wrap; `Logical.spans` maps its
  byte offsets back onto the display rows so only the touched rows light up. A hit reports
  `line` = its first row, so `n`/`N` scrolling and the outline keep their old meaning.
- Known upstream warts in `mermaid-text` (waiting on upstream — do not patch around them locally): after a `gitGraph` merge the commit dot is off by one cell, and `«interface»` in `classDiagram` can overlap a border by one cell.

## Tests

- Shared constructors live in `tests/support/mod.rs`: `render` / `render_no_mermaid` / `plain` / `theme` / `opts` / `args`. The theme is hard-coded dark with mermaid off so snapshots do not drift with CI's `TERM` / `NO_COLOR` — new tests go through these helpers, never `Theme::detect`.
- `temp_md()` gives each test case its own temp directory: parallel cases must not share one `t.md` (it exists so the live-reload tests can write files).
- Snapshots are in `tests/snapshots/` (insta). When render behaviour changes, re-run with `INSTA_UPDATE=always` and review the diff — do not blind-refresh snapshots to make a test pass.
- `examples/demo.gif` (the README's first image) comes from `vhs examples/demo.tape`. vhs 0.12 spells the frame rate `Set Framerate` (there is no `Set FPS`), and the machine needs a CJK font or demo.md renders as tofu boxes. `n` / `N` only step matches after `Enter` — while the query is still being typed they are literal characters, so the tape must `Enter` before `nn`.

## Language

- Code comments and internal docs: Chinese, one line stating the why is enough.
- User-facing strings (`src/cli.rs` help, the status bar, `HELP` in `view.rs`) and `README.md`: English. Chinese docs live separately in `docs/README.zh-CN.md` — when you change one, change the other.

## Releasing

One command: `scripts/tag.sh <version>`. It writes the version into `Cargo.toml` (the `[package]` one — deps use the same key, the script only replaces the first `version = ` line), syncs the root package's version line in `Cargo.lock`, commits, pushes the branch, then tags `v<version>` and pushes the tag. The tag triggers CI: `check` (fmt/clippy/test) → `build` (4 platforms: linux x86_64 + aarch64, macOS arm64, Windows x86_64) → `release` (GitHub Release with the tarballs) → `publish` (crates.io, using the repo secret `CARGO_REGISTRY_TOKEN`) and `tap`.

`tap` is the Homebrew half, so nobody has to remember it after the tag: it re-checks out `main`,
hashes the artifacts `build` just produced (`MDVIEW_ARCHIVE_DIR`, no re-download), regenerates
`Formula/mdview.rb` via `scripts/formula.sh <version>` and pushes one commit. `tag.sh` never
touches the formula, and an Intel macOS bottle never exists (that install path is `cargo install`,
which the formula does not offer). The same script re-run by hand does the same thing from the
published archives.

Don't release by hand (bump → commit → tag). The version has to be right *inside* the tag, because CI builds the tarballs from the tag ref and the publish job only *checks* `tag == Cargo.toml` — it cannot repair a tag afterwards. v0.3.0 was tagged over `Cargo.toml = 0.2.0`, so those release tarballs print `mdview 0.2.0` and crates.io got nothing. Two rules the script encodes: refresh the lock with `cargo metadata`, never `cargo generate-lockfile` (that re-resolves and smuggles unrelated dep bumps — it moved `pest_stack_guard` 1.0.0 → 1.0.1 — into the release commit), and push the branch before the tag (one push with two refspecs can land a tag whose commit never reached the branch).

CI runs `check` + `build` on PRs and on a manual `gh workflow run ci.yml --ref main`; `release` / `publish` are gated on `startsWith(github.ref, 'refs/tags/v')`, so only a `v*` tag can reach crates.io. A plain push to a branch triggers nothing — route work through a PR, or dispatch the workflow to lint `main`. If a tag does ship and `check` goes red, nothing gets published (`release` needs `build`, `build` needs `check`): `git tag -d vX && git push origin :refs/tags/vX`, fix, retag. CI is a copy of the local `fmt` / `clippy` / `test` trio, not a replacement for it — run the trio before pushing.

`cargo publish` runs without `--no-verify` on purpose: it unpacks and rebuilds the tarball, which is a local dry-run.
