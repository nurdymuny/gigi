"""Plot retained observations and engine outputs, not a fitted reconstruction."""
import argparse
import csv
import json
from pathlib import Path
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

parser=argparse.ArgumentParser()
parser.add_argument('directory',type=Path)
args=parser.parse_args()
report=json.loads((args.directory/'report.json').read_text())
fig,axes=plt.subplots(3,2,figsize=(13,12),layout='constrained')
for ax,trace in zip(axes.flat,report['traces']):
    if trace['status'] != 'COMPARED':
        ax.set_title(trace['name']+': NOT COMPARED')
        continue
    values=np.genfromtxt(args.directory/(trace['name']+'-comparison.csv'),delimiter=',',names=True)
    t=(values['time_s']-values['time_s'][0])*1000
    ax.plot(t,values['source_v'],color='#919191',lw=1,label='Measured source')
    ax.plot(t,values['measured_capacitor_v'],color='#177e89',lw=1.5,label='Measured capacitor')
    ax.plot(t,values['gigi_capacitor_v'],color='#bf4254',lw=1.6,label='GIGI, nominal R/C')
    ax.set_title(f"{trace['name']}: R={trace['resistance_ohm']:g} ohm, C={trace['capacitance_f']*1e9:g} nF\n"
                 f"NRMSE {100*trace['normalized_rmse']:.2f}%, max {100*trace['normalized_max_error']:.2f}% of input swing",
                 loc='left',fontsize=11)
    ax.set_xlabel('Elapsed measured time (ms)')
    ax.set_ylabel('Voltage (V)')
    ax.grid(alpha=.2)
    ax.spines[['top','right']].set_visible(False)
handles,labels=axes.flat[0].get_legend_handles_labels()
fig.legend(handles,labels,loc='outside lower center',ncols=3,frameon=False)
fig.suptitle('Six measured RC traces | no parameter fitting\n15,000 observations; diagnostic comparison only. Hardware qualification remains OPEN.',fontsize=16)
path=args.directory/'measured-comparison.png'
fig.savefig(path,dpi=155)
print(path)
