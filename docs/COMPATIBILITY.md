# Compatibility contract

Reference interface: Textualize/rich-cli main at
`46f4d2469097be395558768714a5f07ebccf1412`, retrieved 2026-10-15, reporting version
1.8.0. Differential tests use the released rich-cli 1.8.0 with Rich 12.6.0.
Released 1.8.0 assigns `-j` twice; main assigns JSON `-J`. This implementation
follows main and uses long options in cross-version comparisons.

“Implemented” describes working behavior, not full visual or semantic identity.
The automated comparison corpus is small. Matching it does not establish complete
compatibility with arbitrary Rich renderables or all option combinations.

| Interface | Status and boundaries |
| --- | --- |
| Local files, `-` stdin | Implemented; lossy UTF-8 for files. Stdin currently requires valid UTF-8. |
| HTTP/HTTPS | Rust HTTP/TLS client, redirects, MIME hint, 30 s global timeout, 64 MiB response limit; failed HTTP status is an error. |
| Format autodetection | `.md`, `.json`, `.csv`, `.tsv`, `.rst`, `.ipynb`; URL query strings ignored. Other paths use syntax highlighting. |
| `--syntax`, `-x/--lexer` | Syntect's bundled TextMate grammars, with common aliases. Not all Pygments lexers; unknown explicit lexer is an error. |
| `--theme`, `RICH_THEME` | Syntect themes plus bundled Monokai, Dracula, default and friendly palettes; ansi_dark/ansi_light and solarized aliases. Scope mapping and colors differ from Pygments, including ansi_dark. Unknown themes are errors. |
| `-n/--line-numbers`, `-g/--guides` | Implemented; number gutter and four-column indentation guides. |
| `-h/--head`, `-t/--tail` | Positive line/row counts; mutually exclusive. Syntax and CSV, and code cells in notebooks. Tail intentionally selects the last actual lines rather than upstream's off-by-one selection. |
| `-p/--print` | Nested console markup, named closing tags, `[/]`, escaped opening brackets. Rich's full markup grammar has not been ported. |
| `-j/--emoji` | 3,608 Rich 12.6 shortcode aliases bundled as static data. Variation suffixes such as `:name-text:` are not yet supported. |
| `-s/--style`, `--rule-style`, `-S/--panel-style` | Rich named colors, `#rrggbb`, `rgb(r,g,b)`, `color(n)`, foreground/background; bold, dim, italic, underline, strike, reverse, blink, conceal, negation and links. Frame, encircle, overline, double underline and separate blink2 semantics are not implemented. |
| `-u/--rule`, `--rule-char` | Implemented; styled labels, repeat characters and left/center/right text placement. |
| `-J/--json` | JSON validation, insertion-order-preserving pretty printing and syntax colors. JSON layout uses a fixed two-space indent; Python-specific nonstandard NaN/Infinity values are rejected. |
| `-m/--markdown` | CommonMark through pulldown-cmark: headings, paragraphs, emphasis, links, lists, block quotes, fenced code; GFM tables, task lists, strikethrough, and footnotes. Exact Rich Markdown layout/HTML behavior differs. Table alignments are inferred numerically rather than following Markdown column markers. |
| `--rst` | Subset translated into Markdown: underlined headings, paragraphs, emphasis/literals, lists, code/code-block, note/warning, literal blocks. Arbitrary directives return errors; Sphinx roles, references, substitutions, tables, images and advanced docutils behavior remain incomplete. |
| `--csv` | CSV/TSV/semicolon/pipe detection, quoted/multiline fields, tables, numeric alignment, head/tail, title/caption. Header voting approximates Python Sniffer; complex-number columns, dialect/quote sniffing and malformed-data decisions can differ. |
| `--ipynb` | Code/Markdown/raw cells; execution counts, stream/error/execute-result outputs and plain/Markdown display data. Images/widgets/HTML output are not rendered. ANSI stream styling is stripped rather than preserved. Notebook cell spacing differs. |
| `--inspect` | Explicit error. Upstream evaluates arbitrary Python expressions and introspects live Python objects; that needs a Python runtime and is outside this standalone binary's capabilities. |
| `-l/-r/-c`, `--left/--right/--center` | Block alignment implemented; combinations with fixed/max widths require broader upstream comparisons. |
| `-L/-R/-C/-F`, `--text-*` | Text justification in print mode; full justification expands interword spaces on wrapped lines. |
| `-w/--width`, `-W/--max-width` | Width constraints implemented using terminal size, then COLUMNS, then 80. Extremely narrow content, panels and oversized padding can require more cells than requested. |
| `--soft`, `--no-wrap` | Soft wrapping in print mode; no-wrap crops syntax/print output. Rich's ellipsis/crop decisions are not fully reproduced. |
| `-d/--padding` | CSS-like 1/2/4 nonnegative integers; each value capped at 4096. |
| `-a/--panel`, `-e/--expand`, `--title`, `--caption` | Seven box choices including none, titles/captions and expansion. ascii and ascii2 use the same panel edges. Measurement, wrapping and title elision can differ. |
| `-y/--hyperlinks` | OSC-8 links in Markdown when terminal output is enabled; plain output includes URLs otherwise. Markup links currently also require this flag for terminal emission. |
| `--force-terminal`, pipes | Plain output when redirected; force emits truecolor ANSI. NO_COLOR and TERM=dumb honored unless forced. 16/256-color capability negotiation is not implemented. |
| `-o/--export-html`, `--export-svg` | Escaped standalone exports preserving rendered colors and styles independently of stdout color. Templates, fonts, SVG geometry, and link representation differ from Rich. |
| `--pager` | Native Rust pager: scrolling, page navigation, home/end, mouse wheel, resize redraw, status/scrollbar. Optional PAGER program override, nonterminal fallback. No drag scrollbar or reflow on resize. |
| `--help`, `-v/--version` | Clap help and project version; help layout and branding differ. `-h` remains head. |
| Errors and exit codes | Clap errors exit 2; runtime errors exit 1, stderr diagnostics. Upstream often exits 255. Broken pipes exit successfully for shell composition. |

## Scope of remaining work

A complete compatibility release still requires broader differential fixtures for
styled/ANSI output and combinations of flags, faithful RST parsing, additional
lexer/theme coverage, full Rich markup/style semantics, color capability handling,
and terminal/export visual review. The Python-object inspection requirement needs
an explicit product decision: remove that feature from the compatibility target,
or allow an optional Python runtime. A pure native implementation cannot inspect
arbitrary live Python objects without implementing or embedding Python itself.

The included release workflows, installer and formula generator are implemented.
Only local macOS execution and offline installer-platform selection have been
verified here. Actual Intel macOS, ARM64/x86-64 Linux binaries and the Homebrew tap
must pass their native CI/release checks before publication.
