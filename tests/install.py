#!/usr/bin/env python3
"""Exercise installer and formula generation offline using real archives/checksums."""
import hashlib
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

class InstallTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='richrs-install-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.assets = self.root / 'assets'
        self.assets.mkdir()
        self.bin = self.root / 'tools'
        self.bin.mkdir()
        self.destination = self.root / 'install with spaces'
        self.target = 'aarch64-apple-darwin'
        for target in ['aarch64-apple-darwin','x86_64-apple-darwin',
                       'aarch64-unknown-linux-musl','x86_64-unknown-linux-musl']:
            path = self.assets / f'richrs-v0.1.0-{target}.tar.gz'
            content = b'#!/bin/sh\nprintf "Hello\\n"\n'
            with tarfile.open(path, 'w:gz') as archive:
                entry = tarfile.TarInfo('rich')
                entry.mode = 0o755
                entry.size = len(content)
                archive.addfile(entry, io.BytesIO(content))
        self.generate()
        self.tool('uname', '#!/bin/sh\ncase "$1" in -s) echo "${TEST_OS:-Darwin}";; -m) echo "${TEST_ARCH:-arm64}";; esac\n')
        self.tool('curl', '''#!/bin/sh
# Mock downloads only. All filesystem/checksum/extraction code is real.
url= out=
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out=$2; shift 2;;
    --proto|--tlsv1.2|-w) if [ "$1" = --tlsv1.2 ]; then shift; else shift 2; fi;;
    -*) shift;;
    *) url=$1; shift;;
  esac
done
case "$url" in */latest) printf 'https://github.com/example/richrs/releases/tag/v0.1.0';; *) cp "$TEST_ASSETS/${url##*/}" "$out";; esac
''')
        self.env = dict(os.environ, PATH=f'{self.bin}{os.pathsep}{os.environ["PATH"]}',
                        TEST_ASSETS=str(self.assets), RICHRS_REPOSITORY='example/richrs',
                        RICHRS_VERSION='v0.1.0', RICHRS_INSTALL_DIR=str(self.destination))
    def tool(self, name, content):
        path = self.bin / name
        path.write_text(content)
        path.chmod(0o755)
    def generate(self):
        subprocess.run(['python3', str(ROOT/'scripts/package-release.py'),
                        '--repository','example/richrs','--version','v0.1.0',
                        '--assets',str(self.assets)],check=True,stdout=subprocess.DEVNULL)
    def run_install(self, *args):
        return subprocess.run(['sh',str(ROOT/'install.sh'),*args],env=self.env,capture_output=True,text=True)
    def test_all_platforms(self):
        for system, arch in [('Darwin','arm64'),('Darwin','x86_64'),('Linux','aarch64'),('Linux','x86_64')]:
            with self.subTest(system=system,arch=arch):
                self.env.update(TEST_OS=system,TEST_ARCH=arch)
                run=self.run_install('--force')
                self.assertEqual(run.returncode,0,run.stderr)
                self.assertEqual(subprocess.check_output([str(self.destination/'rich')],text=True),'Hello\n')
    def test_existing_binary_preserved(self):
        self.destination.mkdir()
        (self.destination/'rich').write_text('old')
        self.assertNotEqual(self.run_install().returncode,0)
        self.assertEqual((self.destination/'rich').read_text(),'old')
    def test_checksum_failure_preserves_destination(self):
        path=self.assets/f'richrs-v0.1.0-{self.target}.tar.gz'
        path.write_bytes(path.read_bytes()+b'tampered')
        run=self.run_install()
        self.assertNotEqual(run.returncode,0)
        self.assertIn('checksum verification failed',run.stderr)
        self.assertFalse((self.destination/'rich').exists())
    def test_latest(self):
        self.env['RICHRS_VERSION']='latest'
        run=self.run_install()
        self.assertEqual(run.returncode,0,run.stderr)
    def test_invalid_repository(self):
        for repo in ['owner/repo/extra','../repo','bad repo','owner']:
            self.env['RICHRS_REPOSITORY']=repo
            self.assertNotEqual(self.run_install().returncode,0)
    def test_formula_contains_real_hashes(self):
        formula=(self.assets/'richrs.rb').read_text()
        for asset in self.assets.glob('*.tar.gz'):
            self.assertIn(hashlib.sha256(asset.read_bytes()).hexdigest(),formula)
        self.assertIn('conflicts_with "rich-cli"',formula)
    def test_refuses_missing_release_asset(self):
        next(self.assets.glob('*.tar.gz')).unlink()
        result=subprocess.run(['python3',str(ROOT/'scripts/package-release.py'),
                               '--repository','example/richrs','--version','v0.1.0',
                               '--assets',str(self.assets)],capture_output=True)
        self.assertNotEqual(result.returncode,0)

if __name__ == '__main__':
    unittest.main()
