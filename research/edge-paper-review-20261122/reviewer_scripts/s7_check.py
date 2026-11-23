import json, numpy as np
R='./research/su2-floor-family-20261119/'
d=json.load(open(R+'twist_family_map.json'))
c=[r for r in d['cells'] if r['floor_all']]
print('floor_all',len(c),'x<=0',sum(r['x']<=0 for r in c),'x<0',sum(r['x']<0 for r in c), 'x>0 cells',[(r['x'],r['y'],r['t']) for r in c if r['x']>=0][:10])
print('ts',sorted(set(r['t'] for r in d['cells'])),'nx',len(set(r['x'] for r in d['cells'])),'ny',len(set(r['y'] for r in d['cells'])))
print(d['cells'][0].keys())
# meron
m=json.load(open(R+'test2_meron.json'))
rows=m['rows']; print('meron rows',len(rows),'any floor',any(r['is_floor'] for r in rows),'max res',max(r['residual'] for r in rows),'max Gb',max(r['G_star_b'] for r in rows), 'ts',len(set(r['t'] for r in rows)),'js',sorted(set(r['j'] for r in rows)))
b=json.load(open(R+'test2_berger_control.json'))
print('berger summary', b['summary']['T_all_runs'], {k:v for k,v in b['summary']['per_sector_runs'].items()})
for f in ('su3_test2_family2a.json','su3_test2_family2b.json'):
    s=json.load(open(R+f)); pts=s['points']
    n=0; fl=0; fallback=0; keys=set()
    for p in pts:
        for k,v in p['sectors'].items():
            n+=1; fl+=bool(v.get('is_floor')); keys|=set(v.keys())
            if any('fallback' in kk or 'svd' in kk.lower() for kk in v): pass
    print(f,len(pts),n,fl,keys, s['summary'].get('errors'))
    import re
    txt=open(R+f).read(); print(' fallback mentions', len(re.findall('fallback|gesvd|lapack',txt)))
