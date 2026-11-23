import sys, numpy as np
sys.path.insert(0,'./scripts')
exec(open('s5_rq.py').read().split('for (x,y,t,j)')[0].replace('print(f"j','return (r1,projRQ(em),r2,projRQ(e0)) or print(f"j'))
rng=np.random.default_rng(3); worst=-1e9
for _ in range(25):
    x=rng.uniform(-2,2); y=rng.uniform(0.1,3); t=rng.uniform(0.5,12); j=int(rng.integers(0,30))
    r1,p1,r2,p2=run(x,y,t,j); worst=max(worst,p1-r1,p2-r2)
print('max(projRQ - RQ)',worst)
