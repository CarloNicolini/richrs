#!/usr/bin/env python3
"""Build a Homebrew formula from actual release assets and their SHA256 hashes."""
import argparse
import hashlib
from pathlib import Path
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repository', required=True)
parser.add_argument('--version', required=True)
parser.add_argument('--assets', type=Path, default=Path('dist'))
args = parser.parse_args()
if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', args.repository):
    parser.error('repository must be owner/repo')
if not re.fullmatch(r'v\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?', args.version):
    parser.error('version must be a v-prefixed semantic version')
targets = ['aarch64-apple-darwin', 'x86_64-apple-darwin',
           'aarch64-unknown-linux-musl', 'x86_64-unknown-linux-musl']
hashes = {}
for target in targets:
    archive = args.assets / f'richrs-{args.version}-{target}.tar.gz'
    if not archive.is_file():
        parser.error(f'missing release asset: {archive}')
    hashes[target] = hashlib.sha256(archive.read_bytes()).hexdigest()
base = f'https://github.com/{args.repository}/releases/download/{args.version}'
lines = ['class Richrs < Formula', '  desc "Native terminal formatting utility written in Rust"',
         f'  homepage "https://github.com/{args.repository}"',
         f'  version "{args.version.removeprefix("v")}"', '  license "MIT"',
         '  conflicts_with "rich-cli", because: "both install the rich executable"', '']
for platform, suffix in [('macos', 'apple-darwin'), ('linux', 'unknown-linux-musl')]:
    lines.append(f'  on_{platform} do')
    for arch, target in [('arm', f'aarch64-{suffix}'), ('intel', f'x86_64-{suffix}')]:
        lines.extend([f'    on_{arch} do',
                      f'      url "{base}/richrs-{args.version}-{target}.tar.gz"',
                      f'      sha256 "{hashes[target]}"', '    end'])
    lines.extend(['  end', ''])
lines.extend(['  def install', '    bin.install "rich"', '    bash_completion.install "completions/rich.bash" => "rich"', '    zsh_completion.install "completions/rich.zsh" => "_rich"', '    fish_completion.install "completions/rich.fish"', '  end', '', '  test do',
              '    assert_equal "Hello\\n", shell_output("#{bin}/rich --print Hello")', '  end', 'end', ''])
(args.assets / 'richrs.rb').write_text('\n'.join(lines))
(args.assets / 'SHA256SUMS').write_text(''.join(
    f'{hashes[t]}  richrs-{args.version}-{t}.tar.gz\n' for t in targets))
print(f'Generated {args.assets}/richrs.rb and SHA256SUMS from four release assets')
