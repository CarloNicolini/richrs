# Local verification — 2026-10-15

Host: `macOS-26.5.2-arm64-arm-64bit-Mach-O`. Rust 1.88.0. Reference: rich-cli 1.8.0,
Rich 12.6.0, Python 3.14, isolated under `/private/tmp/richrs-reference`.

- Formatting check and Clippy with warnings denied: passed.
- Rust: 8 unit tests and 22 CLI integration tests passed.
- Offline installer/package-generator tests: 8 passed, including all four platform
  selections, checksum failure, existing binary preservation and real formula hashes.
- Local HTTP/pseudo-terminal tests: 4 passed, including redirects, HTTP errors,
  native pager exit/terminal restoration, broken pipes and color override behavior.
- Large-file stress test: a 32 MiB Rust source file rendered with `--tail 1` using
  5,872 KiB peak resident memory in the optimized release build. The stream path keeps only the current source
  line and requested tail lines in memory, and buffers terminal writes.
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
| markup | 102.42 ms | 5.12 ms | 20.0× |
| syntax | 129.01 ms | 9.37 ms | 13.8× |
| json | 103.00 ms | 7.01 ms | 14.7× |
| source 5000 lines | 1135.06 ms | 234.71 ms | 4.8× |
| json 1000 records | 268.47 ms | 48.83 ms | 5.5× |

The larger source fixture has 5,000 generated Python function definitions. The
larger JSON fixture has 1,000 records. These are synthetic local measurements;
other languages, styles, terminal devices and file sizes need separate benchmarks.
The original Python renderers and Syntect also do different highlighting work.

Reproduce:

```sh
cargo build --release --locked
python3 scripts/compare-upstream.py --reference /private/tmp/richrs-reference/bin/rich --runs 15 --large
```

The current local macOS archive is `dist/richrs-v0.1.0-aarch64-apple-darwin.tar.gz`;
its checksum is recorded in `dist/SHA256SUMS`. It includes all three shell completion
files. This is a local build artifact, not a published release.

## Unverified distribution steps

Intel macOS, both Linux targets, public release URLs, actual Homebrew installation,
macOS signing/notarization, and clean-machine portability remain unverified. No
remote or public release is configured. Configured CI/release jobs must run before
those platforms are described as supported release artifacts. Complete CLI parity
remains open as detailed in `COMPATIBILITY.md`.
