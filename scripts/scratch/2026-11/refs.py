import json, glob
for f in sorted(glob.glob('research/su2-pg-stability-20261117/L*_*.txt')):
    last = open(f).read().strip().splitlines()[-1]
    try:
        d = json.loads(last)
        print(f.split('/')[-1], {k: d[k] for k in ('rough_real_below_minus10', 'min_real_physical', 'max_abs_imag')})
    except Exception as e:
        print(f, 'unparsed', e)
for f in sorted(glob.glob('research/su2-numrange-20261118/*.json')):
    d = json.load(open(f))
    keys = [k for k in d if not isinstance(d[k], (list, dict))]
    print(f.split('/')[-1], {k: d[k] for k in keys[:14]})
