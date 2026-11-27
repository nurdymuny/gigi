"""Mechanically generate isolated broken operators; production files remain untouched."""
import argparse
import json
import os
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[1]
LAB=ROOT/'helike-lab'
TESTS=LAB/'crates/helike-quantum/tests'


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('out_dir',type=Path); args=ap.parse_args()
    args.out_dir.mkdir(parents=True,exist_ok=True)
    source='\n'.join(s for s in (LAB/'crates/helike-quantum/src/enrichment.rs').read_text().splitlines() if not s.startswith('//!'))
    tests=(TESTS/'coupled_enrichment.rs').read_text().replace('use helike_quantum::enrichment::','use crate::enrichment::')
    variants={'baseline':(None,None),
              'drop_forward_coupling':('out[i]+=c[i]*v[n+j]','out[i]+=0.0*c[i]*v[n+j]'),
              'drop_adjoint_coupling':('dot(&self.c[j],&v[..n])+','0.0*dot(&self.c[j],&v[..n])+'),
              'drop_whitening':('/lambda[j].sqrt()','/1.0'),
              'drop_projection':('out.extend(self.columns.iter().map(|u| dot(u,v)))','out.extend(self.columns.iter().map(|u| 0.0*dot(u,v)))')}
    generated=(TESTS/'enrichment_mutation_probe.rs').resolve()
    if generated.parent!=TESTS.resolve() or generated.exists(): raise RuntimeError('refusing to replace existing test')
    results=[]
    for name,(old,new) in variants.items():
        if old and source.count(old)!=1: raise RuntimeError(f'nonunique mutation {name}')
        body=source if old is None else source.replace(old,new)
        text='pub use helike_quantum::{basis,lanczos,model,operator,ritz,support};\nmod enrichment {\n'+body+'\n}\nmod checks {\n'+tests+'\n}\n'
        created=False
        try:
            with generated.open('x') as f:
                created=True; f.write(text)
            run=subprocess.run(['cargo','test','--release','-p','helike-quantum','--test','enrichment_mutation_probe','--','--nocapture'],
                               cwd=LAB,env={**os.environ,'RAYON_NUM_THREADS':'8'},capture_output=True,text=True,timeout=180)
            (args.out_dir/(name+'.log')).write_text(run.stdout+'\n'+run.stderr)
            ran='running 7 tests' in run.stdout
            passed=ran and (run.returncode==0 if name=='baseline' else run.returncode!=0 and 'FAILED' in run.stdout)
            results.append({'variant':name,'compiled_and_ran':ran,'gate_pass':passed,'returncode':run.returncode})
            print(json.dumps(results[-1]),flush=True)
        finally:
            if created: generated.unlink(missing_ok=True)
        if not passed: break
    (args.out_dir/'mutation-results.json').write_text(json.dumps(results,indent=2)+'\n')
    return 0 if len(results)==len(variants) and all(x['gate_pass'] for x in results) else 1


if __name__=='__main__': raise SystemExit(main())
