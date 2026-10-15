# Releasing richrs

The configured GitHub remote has no release or Homebrew tap verified from this
checkout. The release workflow creates a **draft**; publication remains a separate action.

1. Review `COMPATIBILITY.md`, especially limitations that block a drop-in claim.
2. Run formatting, Clippy, Rust tests, `tests/install.py`, and `tests/system.py`.
3. Configure a GitHub repository and push the project. Review CI results on macOS
   first, then Linux. Do not describe configured jobs as verified builds.
4. Set the package version, update the lockfile if needed, and tag exactly
   `v<package.version>`. The workflow rejects mismatched versions.
5. The release jobs test/build natively on Apple Silicon, Intel macOS, x86-64 Linux
   and ARM64 Linux. Linux uses musl. Both Oniguruma and ring build native code;
   inspect dynamic dependencies of the artifacts before claiming portability.
6. Each archive contains Bash, Zsh, and Fish completion scripts. Successful jobs
   upload four archives. The publish job generates `SHA256SUMS`
   and `richrs.rb` from their bytes and creates a draft GitHub release containing
   those files, the archives, and `install.sh`.
7. Download the draft assets, verify checksums and smoke-test on each supported
   system. Test the installer in an isolated prefix and the generated formula
   in a Homebrew tap. Review macOS minimum-version and signing behavior. Artifacts
   are not Developer ID signed or notarized by this workflow.
8. Publish the reviewed release, and add the generated formula to
   `OWNER/homebrew-richrs/Formula/richrs.rb`. Then verify
   `brew install OWNER/richrs/richrs` on macOS and Linuxbrew.

The Rust binary is installed as `rich`; the Homebrew formula is `richrs` and
conflicts with `rich-cli`. This avoids presenting the independent project as an
official Textualize release while retaining CLI invocation compatibility.

## Local packaging

Package the executable with LICENSE, NOTICE and the upstream/data licenses.
The archive must contain a regular file named `rich` at its root. Asset names:

```text
richrs-v0.1.0-aarch64-apple-darwin.tar.gz
richrs-v0.1.0-x86_64-apple-darwin.tar.gz
richrs-v0.1.0-aarch64-unknown-linux-musl.tar.gz
richrs-v0.1.0-x86_64-unknown-linux-musl.tar.gz
```

Once all four real archives are in `dist/`, generate the checksums and formula:

```sh
python3 scripts/package-release.py --repository OWNER/REPOSITORY --version v0.1.0
```

The generator fails if an archive is missing. It does not fabricate platform
artifacts or checksums. The installer tests intentionally use fixture archives;
those must never be published as releases.

Checksums detect corrupted or substituted archive downloads relative to the
manifest. Both downloads use the release host; the workflow does not provide an
independently signed checksum manifest.
