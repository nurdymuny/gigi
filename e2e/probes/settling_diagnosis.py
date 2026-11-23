"""Controlled retrospective diagnostics, never hardware qualification."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import subprocess
import numpy as np
from scipy.linalg import expm
from conduct_settle_live import Engine, ROOT, bundle, conduct
from conduct_settle_measured import parse_scope, configuration, source_hashes

def validate(times, source, initial, tau):
    t,u=np.asarray(times,dtype=float),np.asarray(source,dtype=float)
    if len(t)!=len(u) or len(t)<2 or not np.isfinite([t,u]).all() or not math.isfinite(initial):
        raise ValueError('aligned finite observations required')
    if not math.isfinite(tau) or tau<=0:
        raise ValueError('finite positive tau required')
    if np.any(np.diff(t)<=0):raise ValueError('strictly increasing times required')
    return t,u

def propagate(times, source, initial, tau, mode='linear'):
    t,u=validate(times,source,initial,tau)
    if mode not in ('linear','left','right'):raise ValueError('unknown reconstruction')
    result=[float(initial)]
    for dt,left,right in zip(np.diff(t),u[:-1],u[1:]):
        h=dt/tau
        gain=-math.expm1(-h)
        source_value=right if mode=='right' else left
        value=result[-1]+gain*(source_value-result[-1])
        if mode=='linear':
            ramp_gain=(h/2-h*h/6+h**3/24-h**4/120) if h<1e-3 else 1-gain/h
            value+=(right-left)*ramp_gain
        result.append(float(value))
    return np.array(result)

def tolerance_enclosure(times, source, initial, tau_low, tau_high):
    t,u=validate(times,source,initial,tau_low)
    if not math.isfinite(tau_high) or tau_high<tau_low:raise ValueError('ordered finite tau bounds required')
    low,high=[float(initial)],[float(initial)]
    for dt,left,right in zip(np.diff(t),u[:-1],u[1:]):
        gains=[-math.expm1(-dt/tau_low),-math.expm1(-dt/tau_high)]
        values=[v+g*(s-v) for v in (low[-1],high[-1]) for g in gains for s in (left,right)]
        low.append(min(values));high.append(max(values))
    return np.array(low),np.array(high)

def decompose(observed, original, forced):
    return forced-original,observed-forced

def matrix_oracle(t,u,initial,tau):
    values=[initial]
    for dt,left,right in zip(np.diff(t),u[:-1],u[1:]):
        h=dt/tau
        transition=expm(np.array([[-h,h,0.],[0.,0.,1.],[0.,0.,0.]]))
        values.append(float((transition@np.array([values[-1],left,right-left]))[0]))
    return np.array(values)

def native_left(engine,name,t,u,initial,r,c):
    bundle(engine,name,[dict(id='cap',c=c,v=float(initial))])
    values=[float(initial)]
    for start in range(0,len(t)-1,64):
        stop=min(start+64,len(t)-1)
        durations=np.diff(t[start:stop+1]).tolist()
        phases=[dict(duration_s=dt,configuration=configuration(r,float(u[start+j]))) for j,dt in enumerate(durations)]
        times=np.cumsum(durations).tolist()
        response=conduct(engine,name,phases,times)
        assert response['node_order']==['cap'] and len(response['samples'])==len(durations)
        values.extend(s['voltages_v'][0] for s in response['samples'])
        if stop<len(t)-1:
            engine.ok(f'/v1/bundles/{name}/insert',dict(records=[dict(id='cap',c=c,v=values[-1])]))
    return np.array(values)

def rms(x):return float(np.sqrt(np.mean(x*x)))

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--input',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args();out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    prior=json.loads((args.input/'report.json').read_text())
    manifest=source_hashes()
    assert manifest==prior['source_manifest'],'engine changed; stop controlled comparison'
    binary=ROOT/'target/debug/gigi-stream.exe'
    with (out/'build.log').open('wb') as log:
        subprocess.run(['cargo','build','--features','kahler','--bin','gigi-stream','--target-dir',str(ROOT/'target')],cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest()==prior['binary_sha256']
    report=dict(status='DIAGNOSTIC_ONLY',source_manifest=manifest,binary_sha256=prior['binary_sha256'],
        protocol_sha256=hashlib.sha256((ROOT/'docs/research/settling-diagnosis-protocol-2026-11-13.md').read_bytes()).hexdigest(),
        harness_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),traces=[])
    with Engine(binary,out) as engine:
        for trace in prior['traces']:
            name=trace['name'];raw=(args.input/f'{name}-source.csv').read_bytes()
            assert hashlib.sha256(raw).hexdigest()==trace['sha256']
            t,u,v=parse_scope(raw.decode('utf-8-sig'),trace['resistance_ohm'],trace['capacitance_f'],headers=trace['declared_header_mapping'])
            tau=trace['tau_s']
            original=np.genfromtxt(args.input/f'{name}-comparison.csv',delimiter=',',names=True)['gigi_capacitor_v']
            # Independently recheck the frozen original before attributing its residual.
            from conduct_settle_measured import independent_values
            original_oracle=independent_values(t,trace['phases'],float(v[0]),tau)
            assert np.max(np.abs(original-original_oracle))<1e-9
            predictions={mode:propagate(t,u,v[0],tau,mode) for mode in ('linear','left','right')}
            oracle=matrix_oracle(t,u,v[0],tau)
            matrix_error=float(np.max(np.abs(oracle-predictions['linear'])))
            assert matrix_error<1e-9
            native=native_left(engine,'diagnosis_'+name,t,u,v[0],trace['resistance_ohm'],trace['capacitance_f'])
            native_error=float(np.max(np.abs(native-predictions['left'])))
            assert native_error<1e-9
            low,high=tolerance_enclosure(t,u,v[0],.855*tau,1.155*tau)
            forcing,residual=decompose(v,original,predictions['linear'])
            assert np.max(np.abs((forcing+residual)-(v-original)))<1e-12
            entry=dict(name=name,source_sha256=trace['sha256'],matrix_oracle_error_v=matrix_error,
                native_hold_oracle_error_v=native_error,original_rmse_v=rms(v-original),
                input_reconstruction_rmse_v={k:rms(v-p) for k,p in predictions.items()},events=[])
            for j,event in enumerate(trace['events']):
                p=trace['phases'][j+1]
                mask=(t>=event['switch_time_s']+event['model_bound_s'])&(t<=p['end'])
                if not np.any(mask):
                    entry['events'].append(dict(index=j,status='insufficient_followup',n=0));continue
                indices=np.flatnonzero(mask);worst=int(indices[np.argmax(np.abs(v[mask]-p['source']))])
                outside=np.maximum(low-v,v-high)
                entry['events'].append(dict(index=j,status=event['observation_status'],n=len(indices),
                    target_v=p['source'],tolerance_v=event['tolerance_v'],worst_time_s=float(t[worst]),
                    worst_observed_deviation_v=float(v[worst]-p['source']),
                    worst_original_model_deviation_v=float(original[worst]-p['source']),
                    forcing_contribution_v=float(forcing[worst]),remaining_residual_v=float(residual[worst]),
                    original_postbound_rmse_v=rms(v[mask]-original[mask]),
                    forced_postbound_rmse_v=rms(residual[mask]),
                    source_departure_max_v=float(np.max(np.abs(u[mask]-p['source']))),
                    interpolation_spread_max_v=float(np.max(np.abs(predictions['left'][mask]-predictions['right'][mask]))),
                    outside_tolerance_enclosure_count=int(np.count_nonzero(outside[mask]>1e-9)),
                    outside_tolerance_enclosure_max_v=float(max(0.,np.max(outside[mask])))))
            np.savetxt(out/f'{name}-diagnosis.csv',np.column_stack([t,u,v,original,predictions['linear'],native,low,high]),
                delimiter=',',header='time_s,source_v,observed_v,original_v,linear_forcing_v,native_left_v,tolerance_low_v,tolerance_high_v',comments='')
            report['traces'].append(entry)
            print(json.dumps(entry),flush=True)
    assert source_hashes()==manifest
    (out/'report.json').write_text(json.dumps(report,indent=2),encoding='utf-8')

if __name__=='__main__':main()
