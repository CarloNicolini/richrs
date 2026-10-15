#!/usr/bin/env python3
"""Check that file rendering uses bounded memory on large source inputs."""
import argparse
from pathlib import Path
import resource
import sys
import subprocess
import tempfile

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary',type=Path,default=Path('target/debug/rich'))
parser.add_argument('--mib',type=int,default=32)
args=parser.parse_args()
if args.mib < 1:
    parser.error('--mib must be positive')

with tempfile.TemporaryDirectory(prefix='richrs-stress-') as directory:
    source=Path(directory)/'large.rs'
    line=(b'// stream-check padding padding padding padding padding padding padding padding padding padding padding padding padding padding padding padding padding padding padding padding\n')
    target=args.mib*1024*1024
    with source.open('wb') as output:
        remaining=target
        while remaining > 0:
            chunk=line[:remaining]
            output.write(chunk)
            remaining-=len(chunk)
        output.write(b'\n// LAST_STREAM_SENTINEL\n')
    command=[str(args.binary.resolve()),str(source),'--tail','1']
    child=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    stdout,stderr=child.communicate()
    result=subprocess.CompletedProcess(command,child.returncode,stdout,stderr)
    if result.returncode:
        raise SystemExit(result.stderr)
    if result.stdout != '// LAST_STREAM_SENTINEL\n':
        raise SystemExit(f'unexpected output: {result.stdout[:200]!r}')
    peak=int(resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss)
    if sys.platform=='darwin':
        peak//=1024
    print(f'{args.mib} MiB source, {peak} KiB peak RSS')
    if peak > 256*1024:
        raise SystemExit('streaming RSS exceeded 256 MiB')
