#!/usr/bin/env python3
"""Collect release throughput, allocation/result size and process peak RSS separately."""
import argparse
import hashlib
import json
import platform
from pathlib import Path
import re
import statistics
import subprocess
import time

ROOT=Path(__file__).resolve().parents[2]
DATASETS=['valid','invalid','statements','members','arguments','comment','heredoc','chain','errors','whitespace','newlines','node-limit','token-limit']

def output(command):return subprocess.check_output(command,cwd=ROOT,text=True).strip()
def timed(command):
    clock='/usr/bin/time'
    args=([clock,'-l'] if platform.system()=='Darwin' else [clock,'-v'])+command
    start=time.monotonic()
    result=subprocess.run(args,cwd=ROOT,text=True,capture_output=True,check=True)
    match=re.search(r'(\d+)\s+maximum resident set size',result.stderr) if platform.system()=='Darwin' else re.search(r'Maximum resident set size \(kbytes\): (\d+)',result.stderr)
    rss=int(match[1])*(1 if platform.system()=='Darwin' else 1024) if match else None
    return result.stdout,{'wall_seconds':time.monotonic()-start,'peak_rss_bytes':rss,'resource_report':result.stderr}

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--cold-target',type=Path,help='new empty target directory for cold dependency/library build measurement')
    args=parser.parse_args()
    if args.output.exists():parser.error('output must be a new file')
    if args.cold_target and args.cold_target.exists():parser.error('cold target must not already exist')
    manifest='tools/perf/Cargo.toml'
    subprocess.run(['cargo','build','--release','--locked','--manifest-path',manifest],cwd=ROOT,check=True)
    subprocess.run(['cargo','build','--release','--locked','--manifest-path',manifest,'--features','allocations','--target-dir','tools/perf/target-allocations'],cwd=ROOT,check=True)
    rows=[]
    for dataset in DATASETS:
        for mode in ['lex','parse']:
            stdout,resource=timed(['tools/perf/target/release/pprs-perf',dataset,mode])
            row=json.loads(stdout)
            allocation_stdout,allocation_resource=timed(['tools/perf/target-allocations/release/pprs-perf',dataset,mode])
            allocation=json.loads(allocation_stdout)
            assert len(row['results'])==len(allocation['results'])
            for normal,tracked in zip(row['results'],allocation['results']):
                for key in normal:assert normal[key]==tracked[key],(dataset,mode,key)
            row['allocation_results']=allocation['results']
            row['process']=resource
            row['allocation_process']=allocation_resource
            row['sample_ns_per_batch']=[n/row['iterations_per_sample'] for n in row['sample_ns']]
            row['median_ns_per_batch']=statistics.median(row['sample_ns_per_batch'])
            row['offered_mib_per_second']=row['input_bytes']/(1024*1024)/(row['median_ns_per_batch']/1e9)
            rows.append(row)
            print(dataset,mode,round(row['median_ns_per_batch']/1e6,3),'ms/batch',flush=True)
    builds={}
    if args.cold_target:
        command=['cargo','build','--release','--lib','--locked','--target-dir',str(args.cold_target)]
        _,builds['cold']=timed(command)
        _,builds['warm']=timed(command)
        builds['command']=command
        builds['rlib_bytes']=sum(p.stat().st_size for p in (args.cold_target/'release/deps').glob('libpprs-*.rlib'))
    files=list((ROOT/'src').rglob('*.rs'))+[ROOT/p for p in ['Cargo.toml','Cargo.lock','tools/perf/Cargo.toml','tools/perf/Cargo.lock','tools/perf/src/main.rs','tools/perf/measure.py','tests/fixtures/upstream/cases.jsonl']]
    report={'schema_version':1,'base_commit':output(['git','rev-parse','HEAD']),
        'rustc':output(['rustc','-Vv']),'platform':platform.platform(),
        'cpu':output(['sysctl','-n','machdep.cpu.brand_string']) if platform.system()=='Darwin' else platform.processor(),
        'memory_bytes':int(output(['sysctl','-n','hw.memsize'])) if platform.system()=='Darwin' else None,
        'sources_sha256':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files)},
        'normal_dependency_tree':output(['cargo','tree','--locked','--edges','normal']),
        'builds':builds,'measurements':rows}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(report,indent=2)+'\n')

if __name__=='__main__':main()
