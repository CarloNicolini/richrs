#!/usr/bin/env python3
"""Reproducible plain-output comparison and process-level startup benchmark.

The Python executable is an explicitly supplied *development reference only*.
No Python code is invoked by the Rust binary.
"""
import argparse
import difflib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--reference',required=True,type=Path)
p.add_argument('--binary',type=Path,default=Path('target/release/rich'))
p.add_argument('--output',type=Path,default=Path('docs/verification.json'))
p.add_argument('--runs',type=int,default=15)
p.add_argument('--large',action='store_true',help='Include 5,000-line source and 1,000-record JSON benchmarks')
a=p.parse_args()
env=dict(os.environ,COLUMNS='80',TERM='dumb',PYTHONWARNINGS='ignore::UserWarning')
env.pop('RICH_THEME',None)
env.pop('NO_COLOR',None)
cases=[
    ('markup',['-p','Hello [bold red]world[/]!'],''),
    ('nested markup',['-p','[red]red [b]bold[/b] red[/red]'],''),
    ('escaped markup',['-p',r'\[red]'],''),
    ('emoji',['-p',':rocket: :thumbs_up:','--emoji'],''),
    ('width wrap',['-p','one two three four','-w','7'],''),
    ('right justify',['-p','hello','-w','12','-R'],''),
    ('center justify',['-p','hello','-w','12','-C'],''),
    ('soft wrap',['-p','hello world','-w','4','--soft'],''),
    ('stdin markup',['-p','-'],'[b]hello[/]\n'),
    ('rule',['-u','Hello','-w','30'],''),
    ('custom rule',['-u','Hello','-w','30','--rule-char','='],''),
    ('panel rounded',['-p','Hello','-a','rounded'],''),
    ('panel square',['-p','Hello','-a','square'],''),
    ('panel expanded',['-p','Hello','-a','heavy','-w','24','--title','Title','--caption','Foot'],''),
    ('padding',['-p','Hello','-d','1,2'],''),
    ('syntax',['-','-x','python'],'x = 1\nprint(x)\n'),
    ('syntax head',['-','-x','python','-h','1'],'x = 1\nprint(x)\n'),
    ('syntax tail',['-','-x','python','-t','1'],'x = 1\nprint(x)\n'),
    ('syntax numbers',['-','-x','python','-n'],'x = 1\nprint(x)\n'),
    ('json',['-','--json'],'{"z":1,"a":[true,null]}'),
    ('csv',['-','--csv'],'name,score\nAlice,10\nBob,20\n'),
    ('csv no header',['-','--csv'],'1,2\n3,4\n'),
    ('markdown',['-','-m'],'# Title\n\nHello **world**.\n\n- First\n- Second\n'),
    ('rst',['-','--rst'],'Title\n=====\n\nHello world.\n'),
]
report={'platform':platform.platform(),'reference':str(a.reference),'binary':str(a.binary.resolve()),
        'reference_version':subprocess.check_output([str(a.reference),'--version'],text=True,env=env).strip(),
        'cases':[],'benchmarks':[]}
def run(binary,args,data):
    return subprocess.run([str(binary),*args],input=data,text=True,capture_output=True,env=env)
for name,args,data in cases:
    upstream=run(a.reference,args,data)
    rust=run(a.binary.resolve(),args,data)
    exact=rust.returncode==upstream.returncode and rust.stdout==upstream.stdout
    report['cases'].append(dict(name=name,args=args,input=data,exact=exact,
        rust_status=rust.returncode,upstream_status=upstream.returncode,
        diff='' if exact else ''.join(difflib.unified_diff(upstream.stdout.splitlines(True),rust.stdout.splitlines(True),fromfile='upstream',tofile='rust')),
        rust_stderr=rust.stderr,upstream_stderr=upstream.stderr))
bench_cases=[cases[0],cases[15],cases[19]]
if a.large:
    bench_cases += [
        ('source 5000 lines',['-','-x','python'], ''.join(f'def f_{i}(x): return x + {i}\n' for i in range(5000))),
        ('json 1000 records',['-','--json'],json.dumps([{'id':i,'name':f'item-{i}','active':True} for i in range(1000)])),
    ]
for name,args,data in bench_cases:
    measured={}
    for label,binary in [('upstream',a.reference),('rust',a.binary.resolve())]:
        for _ in range(3):
            result=run(binary,args,data)
            if result.returncode: raise RuntimeError(result.stderr)
        values=[]
        for _ in range(a.runs):
            start=time.perf_counter_ns()
            result=run(binary,args,data)
            elapsed=(time.perf_counter_ns()-start)/1e6
            if result.returncode: raise RuntimeError(result.stderr)
            values.append(elapsed)
        measured[label]={'median_ms':statistics.median(values),'samples_ms':values}
    measured.update(name=name,args=args,input_bytes=len(data.encode()),runs=a.runs,speedup=measured['upstream']['median_ms']/measured['rust']['median_ms'])
    report['benchmarks'].append(measured)
a.output.parent.mkdir(parents=True,exist_ok=True)
a.output.write_text(json.dumps(report,indent=2)+'\n')
print(f"Exact output: {sum(c['exact'] for c in report['cases'])}/{len(cases)} cases")
for case in report['cases']:
    if not case['exact']: print('DIFF:',case['name'])
for b in report['benchmarks']:
    print(f"{b['name']}: upstream {b['upstream']['median_ms']:.2f} ms, Rust {b['rust']['median_ms']:.2f} ms, {b['speedup']:.1f}x")
print(a.output)
