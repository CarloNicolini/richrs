# richrs

A standalone Rust implementation of Textualize's `rich` command-line toolbox.
The executable is named **`rich`**. No Python interpreter, Python package, or
Python library API is required at runtime.

**Status: working implementation, not yet a complete drop-in replacement.**
The command-line compatibility target is
[Textualize/rich-cli](https://github.com/Textualize/rich-cli), which uses the
[Rich library](https://github.com/Textualize/rich) for rendering. This project
implements the CLI rather than translating Rich's Python SDK.
Read [the compatibility matrix](docs/COMPATIBILITY.md) before replacing an
existing installation. In particular, Python-object `--inspect` is unsupported,
reStructuredText is a subset, and syntax colors/layout are not pixel-identical.

## Build and run

Requires Rust 1.88+ and a C compiler for the bundled Oniguruma regex engine.

```sh
cargo build --release --locked
./target/release/rich --help
./target/release/rich --print 'Hello [bold magenta]world[/]! :rocket:' --emoji
./target/release/rich README.md
printf '{"answer":42}\n' | ./target/release/rich - --json
```

Install from this checkout into a user-owned prefix:

```sh
cargo install --path . --locked --root "$HOME/.local"
```

Ensure `$HOME/.local/bin` is on `PATH`. The `rich` name also belongs to the
Python CLI; choose which installation your shell should use with `command -v rich`.

## Examples

```sh
rich src/main.rs --line-numbers --guides --theme dracula
rich src/main.rs --tail 20
rich README.md --hyperlinks --pager
rich data.csv --head 10 --title 'Results'
rich analysis.ipynb
rich 'Build complete' --rule --rule-style green
rich 'Hello [bold]world[/]' --print --panel rounded --padding 1,2 --title Demo
rich README.md --export-html readme.html --export-svg readme.svg
curl -fsSL https://example.com/document.md | rich - --markdown
```

`-` reads stdin. Paths and HTTP(S) URLs select a format by extension; `--syntax`,
`--json`, `--markdown`, `--csv`, `--rst`, and `--ipynb` override that selection.
`--print` and `--rule` interpret their argument as literal console markup.
`-h` means **head**, while help is `--help`. `-J` means JSON and `-j` enables emoji,
following upstream main's correction of the released 1.8.0 short-option collision.

Output to a pipe is plain text unless `--force-terminal` is passed. Terminal
output respects `NO_COLOR` and `TERM=dumb`; explicit `--force-terminal` takes
precedence. Styles remain in HTML/SVG exports even when stdout is plain text.
`RICH_THEME` supplies the default syntax theme.

The native pager supports arrows, j/k, Page Up/Down, Space, Home/End, g/G,
Ctrl-D/Ctrl-U, mouse-wheel scrolling, and q/Escape/Ctrl-C to quit. It restores
the screen and terminal mode on normal exit or a handled error. If `PAGER` is
set, that program is used instead, with shell-style argument splitting and no
shell evaluation. When stdout is redirected, `--pager` prints normally.

## macOS and Linux installation from releases

Release infrastructure is included, but **this checkout has no configured remote
or published release**. The commands below become usable after publishing to your
chosen GitHub repository. `OWNER/REPOSITORY` is a value you must replace.

```sh
curl -fsSL https://raw.githubusercontent.com/OWNER/REPOSITORY/main/install.sh \
  | RICHRS_REPOSITORY=OWNER/REPOSITORY sh
```

The installer defaults to `$HOME/.local/bin`. It selects an Apple Silicon or Intel
macOS binary, or an ARM64/x86-64 Linux musl binary, and verifies its SHA-256 against
the release's `SHA256SUMS` before extraction. It refuses to replace an existing
executable unless `--force` is supplied. It does not need sudo or modify shell files.
Pin a release or change the destination with environment variables:

```sh
RICHRS_REPOSITORY=OWNER/REPOSITORY \
RICHRS_VERSION=v0.1.0 RICHRS_INSTALL_DIR="$HOME/bin" sh install.sh
```

For Homebrew, the release workflow generates **`richrs.rb` with actual archive
checksums** for all four targets. Put that formula in the `Formula/` directory of
your `homebrew-richrs` tap. Users can then run:

```sh
brew install OWNER/richrs/richrs
```

See [release instructions](docs/RELEASING.md) for local packaging and the macOS-first
release process. The repository and tap must exist before either public installer
can work. Homebrew and Linux CI execution have not been validated on this machine.

## Verification and performance

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
python3 tests/install.py
python3 tests/system.py
```

The system tests need permission to open a loopback socket and a pseudo-terminal.
The installer tests are offline and use real archives and checksums with mocked
downloads. CI is configured for macOS and Linux; release builds target both CPU
architectures on each OS. Configuring a workflow is not evidence it has passed.

For differential testing, install **rich-cli 1.8.0** in a separate Python environment:

```sh
python3 -m venv /tmp/richrs-reference
/tmp/richrs-reference/bin/pip install rich-cli==1.8.0
cargo build --release --locked
python3 scripts/compare-upstream.py --reference /tmp/richrs-reference/bin/rich --large
```

[The checked verification report](docs/verification.json) records exact-output
comparisons, mismatches, benchmark samples, and the host. [Local validation](docs/VALIDATION.md)
summarizes 39 passing automated tests and benchmarks. On this Apple Silicon host,
small inputs measured about 14–20× faster, a 5,000-line source fixture 4.6× faster,
and a 1,000-record JSON fixture 5.3× faster than the Python reference. Benchmarks
measure whole processes after warmups and do not establish speedups for all files,
exports, or formats. Python is
used only for development checks and importing bundled data, not by the utility.

## License and attribution

MIT. Independent project, not affiliated with Textualize. Upstream interface and
behavior are credited in [NOTICE](NOTICE). Bundled Rich name tables and translated
Pygments palettes retain their licenses in `docs/`. Rust dependency licenses remain
those of their authors.
