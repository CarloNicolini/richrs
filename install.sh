#!/bin/sh
# Install a checksum-verified richrs release. No root access required.
set -eu

fail() { printf 'richrs: %s\n' "$*" >&2; exit 1; }
repository=${RICHRS_REPOSITORY:-}
version=${RICHRS_VERSION:-latest}
bin_dir=${RICHRS_INSTALL_DIR:-"$HOME/.local/bin"}
force=0
for arg in "$@"; do
    case "$arg" in
        --force) force=1 ;;
        --help) printf '%s\n' 'Usage: install.sh [--force]' 'Set RICHRS_REPOSITORY=owner/repo, optionally RICHRS_VERSION=v0.1.0 and RICHRS_INSTALL_DIR.'; exit 0 ;;
            *) fail "unknown argument: $arg" ;;
    esac
done
[ -n "$repository" ] || fail 'set RICHRS_REPOSITORY=owner/repo to the repository hosting your releases'
case "$repository" in *[!a-zA-Z0-9_./-]*|/*|*/|*..*) fail 'invalid repository' ;; esac
owner=${repository%%/*}
project=${repository#*/}
[ "$owner" != "$repository" ] && [ -n "$owner" ] && [ -n "$project" ] || fail 'repository must be owner/repo'
case "$project" in */*) fail 'repository must be owner/repo' ;; esac
case "$version" in *[!a-zA-Z0-9._-]*|'') fail 'invalid version' ;; esac
for command in curl tar mktemp install; do command -v "$command" >/dev/null 2>&1 || fail "missing required command: $command"; done
case "$(uname -s)" in Darwin) os=apple-darwin ;; Linux) os=unknown-linux-musl ;; *) fail 'supported platforms: macOS and Linux' ;; esac
case "$(uname -m)" in arm64|aarch64) arch=aarch64 ;; x86_64|amd64) arch=x86_64 ;; *) fail 'supported architectures: arm64 and x86_64' ;; esac
if [ "$version" = latest ]; then
    resolved=$(curl --proto '=https' --tlsv1.2 -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$repository/releases/latest")
    version=${resolved##*/}
    case "$version" in v[0-9]*) ;; *) fail 'could not resolve latest release tag' ;; esac
    case "$version" in *[!a-zA-Z0-9._-]*) fail 'invalid release tag' ;; esac
fi
archive="richrs-$version-$arch-$os.tar.gz"
base="https://github.com/$repository/releases/download/$version"
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT HUP INT TERM
curl --proto '=https' --tlsv1.2 -fsSL "$base/$archive" -o "$work_dir/$archive"
curl --proto '=https' --tlsv1.2 -fsSL "$base/SHA256SUMS" -o "$work_dir/SHA256SUMS"
expected=$(awk -v file="$archive" '$2 == file || $2 == "*" file {print $1}' "$work_dir/SHA256SUMS")
[ "${#expected}" = 64 ] || fail 'release checksum is missing or ambiguous'
case "$expected" in *[!a-fA-F0-9]*) fail 'invalid checksum' ;; esac
if command -v sha256sum >/dev/null 2>&1; then
    actual=$(sha256sum "$work_dir/$archive" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
    actual=$(shasum -a 256 "$work_dir/$archive" | awk '{print $1}')
else fail 'sha256sum or shasum is required'; fi
[ "$actual" = "$expected" ] || fail 'checksum verification failed'
tar -xzf "$work_dir/$archive" -C "$work_dir" rich completions
[ -f "$work_dir/rich" ] && [ ! -L "$work_dir/rich" ] || fail 'release must contain a regular rich executable'
shell=${RICHRS_COMPLETION_SHELL:-${SHELL:-unknown}}
shell=${shell##*/}
case "$shell" in
    bash) completion_file=rich.bash; completion_name=rich
        completion_dir=${RICHRS_COMPLETION_DIR:-"${XDG_DATA_HOME:-$HOME/.local/share}/bash-completion/completions"} ;;
    zsh) completion_file=rich.zsh; completion_name=_rich
        completion_dir=${RICHRS_COMPLETION_DIR:-"${ZDOTDIR:-$HOME}/.zfunc"} ;;
    fish) completion_file=rich.fish; completion_name=rich.fish
        completion_dir=${RICHRS_COMPLETION_DIR:-"${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"} ;;
    *) completion_file=; completion_name=; completion_dir= ;;
esac
completion_source=
if [ -n "$completion_file" ] && [ -f "$work_dir/completions/$completion_file" ]; then
    completion_source=$work_dir/completions/$completion_file
fi
if [ -e "$bin_dir/rich" ] || [ -L "$bin_dir/rich" ]; then
    [ "$force" = 1 ] || fail "$bin_dir/rich already exists; use --force to replace it"
    [ ! -d "$bin_dir/rich" ] || fail 'destination is a directory'
fi
if [ -n "$completion_source" ] && { [ -e "$completion_dir/$completion_name" ] || [ -L "$completion_dir/$completion_name" ]; }; then
    [ "$force" = 1 ] || fail "$completion_dir/$completion_name already exists; use --force to replace it"
    [ ! -d "$completion_dir/$completion_name" ] || fail 'completion destination is a directory'
fi
mkdir -p "$bin_dir"
if [ -n "$completion_source" ]; then mkdir -p "$completion_dir"; fi
if [ "$force" = 1 ] && { [ -e "$bin_dir/rich" ] || [ -L "$bin_dir/rich" ]; }; then rm -f "$bin_dir/rich"; fi
install -m 755 "$work_dir/rich" "$bin_dir/rich"
printf 'Installed richrs %s to %s/rich\n' "$version" "$bin_dir"
case ":$PATH:" in *":$bin_dir:"*) ;; *) printf 'Add this directory to PATH: %s\n' "$bin_dir" ;; esac
if [ -n "$completion_source" ]; then
    if [ "$force" = 1 ] && { [ -e "$completion_dir/$completion_name" ] || [ -L "$completion_dir/$completion_name" ]; }; then rm -f "$completion_dir/$completion_name"; fi
    install -m 644 "$completion_source" "$completion_dir/$completion_name"
    printf 'Installed %s completions to %s\n' "$shell" "$completion_dir/$completion_name"
    if [ "$shell" = zsh ]; then printf 'Add "fpath=(\"%s\" $fpath)" to ~/.zshrc to enable this directory.\n' "$completion_dir"; fi
else
    printf 'No completion for shell %s; generate one with rich --generate-completions bash|zsh|fish.\n' "${shell:-unknown}"
fi
