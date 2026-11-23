import json, glob, os
J = os.environ['SCRATCH']
print("== S2d")
for f in sorted(glob.glob(J + '/paper/research/su2-spectrum-n3-20261119/*.json')):
    d = json.load(open(f)); print(os.path.basename(f), {k: d[k] for k in ('rough_real_below_minus10','min_real','W_left','max_abs_imag','C_col')})
print("== S3d (gap to RKMK4 at each sample time, %)")
for f in sorted(glob.glob(J + '/s3d/research/su2-startup-20261119/*.json')):
    n = os.path.basename(f); lv = n[1]; t = n.split('_')[1]
    refN = 64 if lv == '1' else 32
    ref = {round(r['t'],12): r['E'] for r in json.load(open(f"{J}/s3d/research/su2-flow-20261118/L{lv}_{t}_N{refN}_q6.json"))['rows']}
    d = json.load(open(f)); print(n, d['status'], [round(100*abs(r['E']-ref[round(r['t'],12)])/ref[round(r['t'],12)],2) for r in d['rows']], 'E', round(d['E_final'],6) if d['E_final'] else None)
print("== S3c")
for f in sorted(glob.glob(J + '/su2/research/su2-long-window-20261119/*.json')):
    d = json.load(open(f)); print(os.path.basename(f), d['status'], round(d['E_final'],6), round(d['R_final'],4))
