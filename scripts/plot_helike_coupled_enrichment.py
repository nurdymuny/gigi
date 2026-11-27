"""Plot measured results from a completed, independently replayed enrichment receipt."""
import argparse
import json
from pathlib import Path
import numpy as np
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt


def main():
    ap=argparse.ArgumentParser(); ap.add_argument('run_dir',type=Path); args=ap.parse_args()
    receipt=json.loads((args.run_dir/'receipt.json').read_text())
    if receipt['status']!='DIAGNOSTIC_COMPLETE': raise ValueError('no completed diagnostic')
    a=receipt['analysis']; states=a['states']
    old=[states[0]['old_outer_norm'],np.linalg.norm([r['old_outer_norm'] for r in states[1:3]])/np.sqrt(2),states[3]['old_outer_norm']]
    new=[states[0]['new_complement_norm'],np.linalg.norm([r['new_complement_norm'] for r in states[1:3]])/np.sqrt(2),states[3]['new_complement_norm']]
    fig,ax=plt.subplots(1,2,figsize=(11,4.8),layout='constrained')
    colors=['#167d91','#b54845']
    steps=[a[n]['absolute_step'] for n in ('helike','competitor')]
    ax[0].bar(['Helike','Competitor'],steps,color=colors,width=.5)
    ax[0].axhline(.003,color='#333333',linestyle='--',linewidth=1,label='Unchanged step threshold: 0.003')
    ax[0].set_ylim(0,max(.003,max(steps))*1.4)
    for i,s in enumerate(steps): ax[0].text(i,s+max(.003,max(steps))*.035,f'{s:.6f}',ha='center')
    ax[0].set(title='Gap change after four coupled directions',ylabel='Absolute change from Mq 12')
    ax[0].legend(loc='upper left',fontsize=8,frameon=False)
    x=np.arange(3)
    ax[1].bar(x-.18,old,width=.35,label='Before: Mq 12',color='#aab1b3')
    ax[1].bar(x+.18,new,width=.35,label='After: Mq 12 + 4',color='#167d91')
    ax[1].set(xticks=x,xticklabels=['Ground','Helike pair (RMS)','Competitor'],ylabel='Unrepresented residual norm',
              title='Remaining residual within the same Mq 14 probe',ylim=(0,max(old+new)*1.3))
    ax[1].legend(fontsize=8,frameon=False)
    for i,(b,c) in enumerate(zip(old,new)):
        ax[1].text(i,max(b,c)+max(old+new)*.035,f'{100*(1-c/b):.1f}% lower',ha='center',fontsize=9)
    for p in ax:
        p.spines[['top','right']].set_visible(False); p.grid(axis='y',alpha=.15); p.set_axisbelow(True)
    fig.suptitle('One-point coupled enrichment | s=1.18, t=3, g²=0.03',fontsize=13)
    fig.supxlabel('Finite-probe diagnostics only. A small step is not convergence; domain gate not run; full grid held.',fontsize=9)
    fig.savefig(args.run_dir/'enrichment.png',dpi=180)
    fig.savefig(args.run_dir/'enrichment.pdf')
    plt.close(fig)


if __name__=='__main__': main()
