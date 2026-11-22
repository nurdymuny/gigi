import json, numpy as np
R='/Users/bdavis/.claude/jobs/c8f0f39d/tmp/rtc/research/su2-floor-family-20261119/'
for f in ('alignment_test_4b.json','alignment_test_4a.json'):
    d=json.load(open(R+f)); pts=d['points']
    print(f, len(pts), pts[0]['s'], pts[-1]['s'], d['summary']['per_sector_runs'])
    print(' keys', [k for k in pts[0] if k!='sectors'])
    for p in pts[:1]+pts[-1:]:
        print({k:v for k,v in p.items() if k!='sectors'})
    floors={}
    for p in pts:
        for k,v in p['sectors'].items():
            if v['is_floor']: floors.setdefault(k,[]).append(p['s'])
    print(' floors', {k:(min(v),max(v),len(v)) for k,v in floors.items()})
    k1='1' if '1' in pts[0]['sectors'] else '(1, 0)'
    g=[p['sectors'][k1]['extremal']-p['sectors'][k1]['slice_floor'] for p in pts]
    print(' gap',k1, g[0], g[-1], min(g), max(g))
d=json.load(open(R+'controls.json')) if False else None
s=json.load(open(R+'stretch_test.json'))
print(s['summary']['agreement_table'])
pts=s['points']; print(len(pts), [k for k in pts[0] if k!='sectors'])
from collections import Counter
print(Counter(p.get('class', p.get('label')) for p in pts))
allf=[p for p in pts if all(v['is_floor'] for v in p['sectors'].values())]; print('all floor',len(allf))
for p in pts:
    if abs(p['s']-1.5)<1e-9 and abs(p['t']-3)<1e-9:
        print({k:v['extremal']-v['slice_floor'] for k,v in p['sectors'].items()}, {k:v for k,v in p.items() if k!='sectors'})
    if abs(p['s']-1.5)<1e-9: print('t',p['t'], p['sectors']['1']['extremal']-p['sectors']['1']['slice_floor'])
sing=[p for p in pts if p.get('class',p.get('label'))=='L1']
print('L1 floors', Counter(tuple(k for k,v in p['sectors'].items() if v['is_floor']) for p in sing))
