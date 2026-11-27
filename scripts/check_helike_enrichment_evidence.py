"""Replay the actual saved result and reject deliberate corruptions in memory."""
import argparse
import copy
import json
from pathlib import Path
import run_helike_coupled_enrichment as check


def audit(args):
    receipt=json.loads((args.run_dir/'receipt.json').read_text())
    raw=args.run_dir/'enriched.json'
    native=json.loads((args.run_dir/'native-receipt.json').read_text())
    check.require(receipt['status']=='DIAGNOSTIC_COMPLETE','incomplete run')
    check.require(receipt['enrichment_sha256_lf']==check.grid.sha(raw)==native['results']['enrichment_sha256_lf'],'changed export')
    check.require(receipt['native_receipt_sha256_lf']==check.grid.sha(args.run_dir/'native-receipt.json'),'changed native receipt')
    check.require(check.grid.sha(check.SOURCE)==check.SOURCE_SHA and check.grid.sha(check.PARENT)==check.PARENT_SHA,'changed inputs')
    check.require(receipt['input_sha256_lf']==check.SOURCE_SHA and receipt['parent_sha256_lf']==check.PARENT_SHA,'changed lineage')
    original=json.loads(raw.read_text()); old=json.loads(check.SOURCE.read_text())
    replay=check.inspect(original,old)
    check.require(replay==receipt['analysis'],'recorded analysis differs from replay')
    variants={
        'false_grid_release':lambda x:x.update(full_grid_released=True),
        'relaxed_solver_tolerance':lambda x:x['solver'].update(tol=1e-4),
        'erased_complement':lambda x:x['states'][3].update(outside_norm=0.),
        'zero_direction':lambda x:x['lift_columns_bits'].__setitem__(0,[0]*38760),
        'zero_state':lambda x:x['vectors_bits'].__setitem__(0,[0]*18568),
        'wrong_retained_identity':lambda x:x['lift_positions'].__setitem__(0,x['lift_positions'][1]),
        'forged_gap':lambda x:x['identification']['competitor'].update(gap=0.),
        'substitute_dressed_overlap':lambda x:x['identification']['competitor'].update(
            dressed_weight=x['identification']['competitor']['dressed_overlap']),
    }
    rows=[]
    for name,mutate in variants.items():
        x=copy.deepcopy(original); mutate(x)
        try:
            check.inspect(x,old)
        except ValueError as error:
            rows.append({'variant':name,'rejected':True,'reason':str(error)})
        else:
            rows.append({'variant':name,'rejected':False})
        print(json.dumps(rows[-1]),flush=True)
    passed=all(r['rejected'] for r in rows)
    result={'status':'CHECKED' if passed else 'STOPPED','baseline_replay':'PASS','mutations':rows,'full_grid_released':False}
    args.out.write_text(json.dumps(result,indent=2)+'\n')
    return 0 if passed else 1


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('run_dir',type=Path); ap.add_argument('out',type=Path); args=ap.parse_args()
    protected={args.run_dir/name for name in ('receipt.json','native-receipt.json','enriched.json','receipt-integrity.json')}
    if args.out.resolve() in {p.resolve() for p in protected}:
        raise ValueError('audit output must not replace source evidence')
    args.out.write_text(json.dumps({'status':'RUNNING','full_grid_released':False})+'\n')
    try:
        return audit(args)
    except BaseException as error:
        args.out.write_text(json.dumps({'status':'STOPPED','full_grid_released':False,'error':f'{type(error).__name__}: {error}'},indent=2)+'\n')
        print(f'Audit stopped: {error}',flush=True)
        return 2


if __name__=='__main__': raise SystemExit(main())
