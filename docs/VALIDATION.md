# Local verification — 2026-10-15

Host: `macOS-26.5.2-arm64-arm-64bit-Mach-O`. Rust 1.88.0. Reference: rich-cli 1.8.0,
Rich 12.6.0, Python 3.14, isolated under `/private/tmp/richrs-reference`.

- Formatting check and Clippy with warnings denied: passed.
- Rust: 8 unit tests and 20 CLI integration tests passed.
- Offline installer/package-generator tests: 7 passed, including all four platform
  selections, checksum failure, existing binary preservation and real formula hashes.
- Local HTTP/pseudo-terminal tests: 4 passed, including redirects, HTTP errors,
  native pager exit/terminal restoration, broken pipes and color override behavior.
- Release build: passed on Apple Silicon macOS. `file` identifies a native arm64
  Mach-O executable. `otool -L` lists only system libiconv and libSystem.
- Packaged native archive: extracted into a temporary directory and successfully
  executed with styled markup. Its SHA256 manifest is in `dist/SHA256SUMS`.
- Differential corpus: 23/24 plain-output cases match exactly. The mismatch is
  intentional: upstream tail selects a trailing blank line instead of the last
  actual line. Full diffs, inputs and samples are in `verification.json`.

The 39 automated tests are distinct from the 24 comparison cases. These counts
are coverage evidence for this implementation, not proof of complete compatibility.

## Process benchmarks

Warm cache; 3 warmups and 15 timed subprocess executions per implementation and
case. Same input and flags, stdout/stderr captured, width 80, terminal color off.
Medians include process startup, rendering and captured output. No network fetches
or interactive paging are timed.

| Input | Python upstream | Rust | Speedup |
| --- | ---: | ---: | ---: |
| markup | 107.34 ms | 5.47 ms | 19.6× |
| syntax | 133.33 ms | 9.56 ms | 13.9× |
| json | 106.33 ms | 7.18 ms | 14.8× |
| source 5000 lines | 1136.49 ms | 245.11 ms | 4.6× |
| json 1000 records | 260.13 ms | 49.12 ms | 5.3× |

The larger source fixture has 5,000 generated Python function definitions. The
larger JSON fixture has 1,000 records. These are synthetic local measurements;
other languages, styles, terminal devices and file sizes need separate benchmarks.
The original Python renderers and Syntect also do different highlighting work.

Reproduce:

```sh
cargo build --release --locked
python3 scripts/compare-upstream.py --reference /private/tmp/richrs-reference/bin/rich --runs 15 --large
```

Binary SHA256: `33ecef9c235bbd279b720c63e293da59bfb901051d151675511118c34a5304ff`.

## Unverified distribution steps

Intel macOS, both Linux targets, public release URLs, actual Homebrew installation,
macOS signing/notarization, and clean-machine portability remain unverified. No
remote or public release is configured. Configured CI/release jobs must run before
those platforms are described as supported release artifacts. Complete CLI parity
remains open as detailed in `COMPATIBILITY.md`.
