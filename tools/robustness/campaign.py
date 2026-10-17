#!/usr/bin/env python3
"""Run bounded libFuzzer/ASan campaigns and retain commands, logs and evidence."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
sys.dont_write_bytecode = True
import seed

ROOT=Path(__file__).resolve().parents[2]

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,required=True,help='new directory for evidence and deterministic initial corpora')
    parser.add_argument('--seconds',type=int,default=120)
    parser.add_argument('--toolchain',default='nightly')
    parser.add_argument('--bootstrap-stable',action='store_true',help='explicit experimental fallback when nightly cannot be installed')
    args=parser.parse_args()
    if args.seconds<1:parser.error('seconds must be positive')
    if args.output.exists():parser.error('output must be a new directory')
    args.output.mkdir(parents=True)
    env=os.environ.copy()
    if args.bootstrap_stable:env['RUSTC_BOOTSTRAP']='1'
    toolchain='stable' if args.bootstrap_stable else args.toolchain
    files=list((ROOT/'src').rglob('*.rs'))+list((ROOT/'fuzz/fuzz_targets').glob('*.rs'))
    files += [ROOT/p for p in ['Cargo.toml','Cargo.lock','fuzz/Cargo.toml','fuzz/Cargo.lock','tests/support/invariants.rs','tests/support/generated.rs','tests/fixtures/upstream/cases.jsonl','tools/robustness/seed.py']]
    hashes={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files)}
    byte_seeds={hashlib.sha256(s).hexdigest():s for s in seed.seeds()}
    rows=[]
    for index,target in enumerate(['lex_bytes','parse_bytes','parse_generated']):
        corpus=args.output/'corpus'/target;corpus.mkdir(parents=True)
        sources=byte_seeds if index<2 else {str(i):bytes([i,i//2,i%7,255-i])*4 for i in range(256)}
        for name,data in sources.items():(corpus/name).write_bytes(data)
        command=['cargo','+'+toolchain,'fuzz','run',target,str(corpus.resolve()),'--',f'-max_total_time={args.seconds}',f'-seed={12001+index}','-max_len='+('128' if index==2 else '65536'),'-timeout=10','-rss_limit_mb=2048','-print_final_stats=1']
        start=time.monotonic()
        with (args.output/f'{target}.log').open('w') as log:
            try:
                result=subprocess.run(command,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=args.seconds+180)
                code=result.returncode
            except subprocess.TimeoutExpired:
                code='driver-timeout'
        log=(args.output/f'{target}.log').read_text()
        done=re.search(r'Done (\d+) runs in (\d+) second',log)
        row={'target':target,'command':command,'returncode':code,'elapsed_seconds':time.monotonic()-start,
             'initial_files':len(sources),'initial_sha256':hashlib.sha256(''.join(sorted(hashlib.sha256(v).hexdigest() for v in sources.values())).encode()).hexdigest(),
             'runs':int(done[1]) if done else None,'fuzz_seconds':int(done[2]) if done else None,
             'last_coverage_line':next((line for line in reversed(log.splitlines()) if ' DONE ' in line.replace('\t',' ')),None),
             'log_sha256':hashlib.sha256(log.encode()).hexdigest()}
        rows.append(row)
        report={'schema_version':1,'base_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                'bootstrap_stable':args.bootstrap_stable,'toolchain':subprocess.check_output(['rustc','+'+toolchain,'-Vv'],text=True),
                'cargo_fuzz':subprocess.check_output(['cargo','fuzz','--version'],text=True).strip(),'sources_sha256':hashes,'runs':rows}
        (args.output/'campaign.json').write_text(json.dumps(report,indent=2)+'\n')
        print(json.dumps(row),flush=True)
        if code!=0 or not done:raise SystemExit(1)

if __name__=='__main__':main()
