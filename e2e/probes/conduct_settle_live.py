"""Live passive-network gates. Owns an isolated process; never touches production.

Requires numpy/scipy. Results, requests, responses and source hashes are retained.
Run from the repository: python e2e/probes/conduct_settle_live.py --binary ...
"""
from __future__ import annotations
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import numpy as np
from scipy.linalg import expm
from scipy.integrate import quad

ROOT = Path(__file__).resolve().parents[2]
FIELDS = dict(id="id", capacitance="c", voltage="v")

class Engine:
    def __init__(self, binary, directory):
        self.binary, self.directory = binary.resolve(), directory.resolve()
        self.receipts = []
        self.process = None
    def __enter__(self):
        with socket.socket() as s:
            s.bind(("127.0.0.1", 0))
            port = s.getsockname()[1]
        self.base = f"http://127.0.0.1:{port}"
        env = {k:v for k,v in os.environ.items() if not k.startswith(("GIGI_", "TIGRIS_", "AWS_"))}
        env.update(PORT=str(port), GIGI_DATA_DIR=str(self.directory/"data"), GIGI_SKIP_BOOT_SNAPSHOT="1")
        self.log = (self.directory/"server.log").open("wb")
        self.process = subprocess.Popen([str(self.binary)], cwd=self.directory, env=env,
            stdout=self.log, stderr=self.log, creationflags=getattr(subprocess,"CREATE_NO_WINDOW",0))
        try:
            for _ in range(600):
                if self.process.poll() is not None: raise RuntimeError("owned server exited; see server.log")
                try:
                    status, body = self.call("/v1/health")
                    if status == 200 and not body.get("loading", False): return self
                except OSError: pass
                time.sleep(.2)
            raise RuntimeError("owned server readiness timeout")
        except BaseException:
            self.__exit__(None,None,None)
            raise
    def __exit__(self,*args):
        if self.process and self.process.poll() is None:
            self.process.terminate(); self.process.wait(timeout=30)
        self.log.close()
        (self.directory/"receipts.json").write_text(json.dumps(self.receipts, indent=2), encoding="utf-8")
    def call(self,path,body=None,accept="application/json"):
        req=urllib.request.Request(self.base+path, data=None if body is None else json.dumps(body,allow_nan=False).encode(),
            headers={"Content-Type":"application/json","Accept":accept})
        try: response=urllib.request.urlopen(req,timeout=60)
        except urllib.error.HTTPError as error: response=error
        with response:
            raw=response.read().decode()
            try: value=json.loads(raw)
            except json.JSONDecodeError: value=raw
            self.receipts.append(dict(path=path,request=body,accept=accept,status=response.status,headers=dict(response.headers),response=value))
            return response.status,value
    def ok(self,path,body=None):
        status,value=self.call(path,body)
        assert status in (200,201),(path,status,value)
        return value

def bundle(engine,name,nodes):
    engine.ok("/v1/bundles",dict(name=name,schema=dict(fields={"id":"text","c":"numeric","v":"numeric"},keys=["id"])))
    engine.ok(f"/v1/bundles/{name}/insert",dict(records=nodes))

def conduct(engine,name,phases,times):
    return engine.ok(f"/v1/bundles/{name}/physics/conduct",dict(units="SI",fields=FIELDS,phases=phases,sample_times_s=times))

def settle(engine,name,config,weights,tolerance,deadline):
    return engine.ok(f"/v1/bundles/{name}/physics/settle",dict(units="SI",fields=FIELDS,configuration=config,readout=weights,tolerance_v=tolerance,deadline_s=deadline))

def oracle(nodes,config,initial,t):
    # Independent nonsymmetric affine generator; scipy expm, not symmetric modes.
    n=len(nodes); names={r["id"]:i for i,r in enumerate(nodes)}
    k=np.zeros((n,n)); forcing=np.zeros(n)
    for edge in config["edges"]:
        i,j=names[edge["source"]],names[edge["target"]];g=edge["conductance_s"]
        k[i,i]+=g;k[j,j]+=g;k[i,j]-=g;k[j,i]-=g
    for p in config["ports"]:
        i=names[p["node"]];k[i,i]+=p["conductance_s"];forcing[i]+=p["conductance_s"]*p["voltage_v"]
    c=np.array([r["c"] for r in nodes]); generator=np.zeros((n+1,n+1))
    generator[:n,:n]=-k/c[:,None];generator[:n,n]=forcing/c
    return (expm(generator*t)@np.r_[initial,1])[:n]

def math_gates(engine):
    nodes=[dict(id="a",c=1.,v=3.),dict(id="b",c=1.,v=1.)]
    bundle(engine,"cs_exact",nodes)
    cfg=dict(edges=[dict(source="a",target="b",conductance_s=2.)],ports=[])
    result=conduct(engine,"cs_exact",[dict(duration_s=2.,configuration=cfg)],[0.,.1,.7,2.])
    for s in result["samples"]:
        e=math.exp(-4*s["time_s"])
        np.testing.assert_allclose(s["voltages_v"],[2+e,2-e],rtol=1e-10,atol=1e-12)
        assert abs(s["dissipated_j"]-(1-e*e))<1e-10
    s=settle(engine,"cs_exact",cfg,{"a":1.,"b":-1.},.1,.7)
    assert not s["meets_deadline"]
    assert abs(s["settling_bound_s"]-math.log(20)/4)<1e-10
    assert settle(engine,"cs_exact",cfg,{"a":1.,"b":-1.},.1,.8)["meets_deadline"]
    # Unequal storage, multiple source ports, repeated switching, open intervals.
    nodes=[dict(id=str(i),c=c,v=v) for i,(c,v) in enumerate([(1e-6,3.),(2e-6,0.2),(7e-7,-.1),(3e-6,1.)])]
    bundle(engine,"cs_multi",nodes)
    cfg=dict(edges=[dict(source=str(i),target=str(i+1),conductance_s=g) for i,g in enumerate([.001,.0003,.002])],ports=[dict(node="0",conductance_s=.0005,voltage_v=1.),dict(node="3",conductance_s=.001,voltage_v=2.)])
    cfg2=dict(edges=cfg["edges"][:1],ports=[dict(node="2",conductance_s=.002,voltage_v=-.2)])
    phases=[dict(duration_s=.00137,configuration=cfg),dict(duration_s=.00222,configuration=cfg2),dict(duration_s=.003,configuration=cfg)]
    times=np.linspace(0,sum(p["duration_s"] for p in phases),75).tolist()
    result=conduct(engine,"cs_multi",phases,times)
    starts=[0.,.00137,.00359];states=[np.array([r["v"] for r in nodes])]
    for p in phases[:-1]:states.append(oracle(nodes,p["configuration"],states[-1],p["duration_s"]))
    max_error=0.; lost=0.;supplied=0.;offsets=[]
    names={r["id"]:i for i,r in enumerate(nodes)}
    def power(t,phase,initial):
        v=oracle(nodes,phase,initial,t)
        d=sum(e["conductance_s"]*(v[names[e["source"]]]-v[names[e["target"]]])**2 for e in phase["edges"])
        d+=sum(p["conductance_s"]*(p["voltage_v"]-v[names[p["node"]]])**2 for p in phase["ports"])
        w=sum(p["voltage_v"]*p["conductance_s"]*(p["voltage_v"]-v[names[p["node"]]]) for p in phase["ports"])
        return d,w
    for p,v in zip(phases,states):
        offsets.append((lost,supplied))
        lost+=quad(lambda t:power(t,p["configuration"],v)[0],0,p["duration_s"],epsabs=1e-16)[0]
        supplied+=quad(lambda t:power(t,p["configuration"],v)[1],0,p["duration_s"],epsabs=1e-16)[0]
    for row in result["samples"]:
        i=row["phase_index"];t=row["time_s"]-starts[i];cfg=phases[i]["configuration"]
        expected=oracle(nodes,cfg,states[i],t)
        max_error=max(max_error,float(np.max(abs(np.array(row["voltages_v"])-expected))))
        for j,key in enumerate(("dissipated_j","supplied_j")):
            expected_energy=offsets[i][j]+quad(lambda t:power(t,cfg,states[i])[j],0,t,epsabs=1e-16)[0]
            assert abs(row[key]-expected_energy)<=1e-9*max(1e-12,abs(expected_energy)),(key,row[key],expected_energy)
        assert abs(row["balance_residual_j"])<1e-14
    assert max_error<1e-9,max_error
    # Two decays cancel at t=0, then depart. Verify the bound, not that crossing.
    crossing_nodes=[dict(id="a",c=1.,v=1.),dict(id="b",c=1.,v=-1.)]
    bundle(engine,"cs_crossing",crossing_nodes)
    crossing_cfg=dict(edges=[],ports=[dict(node="a",conductance_s=1.,voltage_v=0.),dict(node="b",conductance_s=2.,voltage_v=0.)])
    crossing=settle(engine,"cs_crossing",crossing_cfg,{"a":1.,"b":1.},.01,0.)
    assert not crossing["meets_deadline"] and crossing["settling_bound_s"]>4.6
    beyond=np.linspace(crossing["settling_bound_s"],crossing["settling_bound_s"]+20,1001)
    future_errors=[abs(oracle(crossing_nodes,crossing_cfg,np.array([1.,-1.]),t).sum()) for t in beyond]
    assert max(future_errors)<=.01
    return dict(two_store_settling_s=s["settling_bound_s"],multi_node_max_voltage_error=max_error,samples_checked=len(times),future_bound_samples=len(beyond),false_crossing_rejected=True)

def wire_gates(engine):
    cfg=dict(edges=[],ports=[dict(node="a",conductance_s=1.,voltage_v=1.)])
    bundle(engine,"cs_wire",[dict(id="a",c=1.,v=0.)])
    req=dict(units="SI",fields=FIELDS,phases=[dict(duration_s=1.,configuration=cfg)],sample_times_s=[0.,.5,1.])
    path="/v1/bundles/cs_wire/physics/conduct"
    first=engine.ok(path,req)
    status,encoded=engine.call(path,req,"application/dhoom")
    assert status==200 and isinstance(encoded,str) and encoded.startswith("brain_response"),(status,encoded)
    # Exact decoding is separately checked with GIGI's real DHOOM decoder.
    settle_req=dict(units="SI",fields=FIELDS,configuration=cfg,readout={"a":1.},tolerance_v=.01,deadline_s=1.)
    engine.ok("/v1/bundles/cs_wire/physics/settle",settle_req)
    assert engine.call("/v1/bundles/cs_wire/physics/settle",settle_req,"application/dhoom")[0]==200
    for change,needle in [({"units":"mV"},"SI"),({"typo":1},"unknown field"),({"fields":dict(FIELDS,voltage="missing")},"missing"),({"sample_times_s":[.5,.5]},"strictly increasing")]:
        status,value=engine.call(path,dict(req,**change))
        assert status==422 and needle in value["error"],(status,value)
    bad=dict(edges=[],ports=[dict(node="a",conductance_s=-1.,voltage_v=1.)])
    status,value=engine.call(path,dict(req,phases=[dict(duration_s=1.,configuration=bad)]))
    assert status==422 and "conductance_s" in value["error"]
    assert engine.call("/v1/bundles/nonexistent_cs/physics/conduct",req)[0]==404
    engine.ok("/v1/bundles/cs_wire/insert",dict(records=[dict(id="a",c=1.,v=2.)]))
    changed=engine.ok(path,req)
    assert changed["source"]["revision"]>first["source"]["revision"]
    assert changed["samples"][0]["voltages_v"]==[2.]
    catalog=engine.ok("/v1/openapi.json")
    for verb in ("conduct","settle"):assert f"/v1/bundles/{{name}}/physics/{verb}" in catalog["paths"]
    return dict(refusals_checked=6,source_revision_changed=True,dhoom_receipts=2)

def network_sizes(engine):
    rng=np.random.default_rng(20261113)
    max_error=0.;cases=0
    for size in [1,2,5,9,16,32]:
        for anchored in [False,True]:
            nodes=[dict(id=f"n{i:02}",c=float(rng.uniform(.5e-6,2e-6)),v=float(rng.uniform(-2.,2.))) for i in range(size)]
            cfg=dict(edges=[dict(source=nodes[i]["id"],target=nodes[i+1]["id"],conductance_s=float(rng.uniform(.0005,.002))) for i in range(size-1)],ports=[])
            if anchored:cfg["ports"]=[dict(node="n00",conductance_s=.001,voltage_v=1.5)]
            name=f"cs_size_{size}_{int(anchored)}";bundle(engine,name,nodes)
            times=[0.,1e-7,.0001,.001,.01,.1,.2]
            result=conduct(engine,name,[dict(duration_s=.2,configuration=cfg)],times)
            initial=np.array([n["v"] for n in nodes]);cap=np.array([n["c"] for n in nodes])
            for sample in result["samples"]:
                actual=np.array(sample["voltages_v"])
                expected=oracle(nodes,cfg,initial,sample["time_s"])
                error=float(max(abs(actual-expected)));max_error=max(max_error,error)
                assert error<1e-9,(name,error)
                lower=min(initial.min(),1.5) if anchored else initial.min()
                upper=max(initial.max(),1.5) if anchored else initial.max()
                assert actual.min()>=lower-1e-10
                assert actual.max()<=upper+1e-10
                if not anchored:assert abs(cap@actual-cap@initial)<1e-10*sum(cap)
            cases+=1
    return dict(seed=20261113,networks=cases,sizes=[1,2,5,9,16,32],max_voltage_error=max_error)

DATA_REV="16c17c8fcaef93c5faecd2f178b0eb24f5f3b187"
DATA_URL=f"https://raw.githubusercontent.com/xsoul-code/esp32-rc-signal-processing/{DATA_REV}/raw_values.csv"
DATA_SHA="3283810fa8b161c6b8e441f19750e63c8e0574bd01baba250298808e290e9fa1"

def sensor_replay(engine,out):
    raw=urllib.request.urlopen(DATA_URL,timeout=60).read()
    assert hashlib.sha256(raw).hexdigest()==DATA_SHA,"upstream data changed"
    counts=[float(row) for row in raw.decode().splitlines()]
    assert len(counts)==260 and all(0<=x<=4095 and x.is_integer() for x in counts)
    engine.ok("/v1/bundles",dict(name="cs_measured_counts",schema=dict(
        fields={"seq":"numeric","adc_count":"numeric","source_sha256":"text"},keys=["seq"])))
    observed=[dict(seq=i,adc_count=x,source_sha256=DATA_SHA) for i,x in enumerate(counts)]
    engine.ok("/v1/bundles/cs_measured_counts/insert",dict(records=observed))
    stored=engine.ok("/v1/bundles/cs_measured_counts/query",dict(sort_by="seq",limit=500))
    assert stored["data"]==observed,"source observations must survive storage unchanged"
    counts=[row["adc_count"] for row in stored["data"]]
    # These conversions and components are an application-declared hypothetical
    # front-end, not a calibration or model of the source author's ESP32 circuit.
    nodes=[dict(id="reference",c=1e-10,v=0.),dict(id="sample",c=1e-9,v=0.),dict(id="sensor",c=1e-10,v=0.)]
    bundle(engine,"cs_sensor_replay",nodes)
    tolerance=3.3/8192.; decisions=[];max_error=0.
    oracle_state=np.array([n["v"] for n in nodes])
    for row,count in enumerate(counts):
        source_v=count*3.3/4095.
        for channel in ("sensor","reference"):
            ports=[dict(node="sensor",conductance_s=1e-5,voltage_v=source_v),dict(node="reference",conductance_s=1e-3,voltage_v=0.)]
            off=dict(edges=[],ports=ports)
            before=np.array([n["v"] for n in nodes])
            opened=conduct(engine,"cs_sensor_replay",[dict(duration_s=1e-6,configuration=off)],[1e-6])
            oracle_off=oracle(nodes,off,oracle_state,1e-6)
            actual=np.array(opened["samples"][0]["voltages_v"])
            max_error=max(max_error,float(max(abs(actual-oracle_off))))
            for n,v in zip(nodes,actual):n["v"]=float(v)
            engine.ok("/v1/bundles/cs_sensor_replay/insert",dict(records=nodes))
            cfg=dict(edges=[dict(source=channel,target="sample",conductance_s=.01)],ports=ports)
            target=source_v if channel=="sensor" else 0.
            bound=settle(engine,"cs_sensor_replay",cfg,{"sample":1.},tolerance,.002)
            assert abs(bound["readout_equilibrium_v"]-target)<1e-9
            wait=bound["settling_bound_s"]*1.001+1e-12
            query_times=sorted(set([20e-6,wait]))
            before=np.array([n["v"] for n in nodes])
            run=conduct(engine,"cs_sensor_replay",[dict(duration_s=max(query_times),configuration=cfg)],query_times)
            assert len(run["samples"])==len(query_times)
            samples={}
            for requested,sample in zip(query_times,run["samples"]):
                assert abs(sample["time_s"]-requested)<=2*math.ulp(requested)
                samples[requested]=sample
            expected=oracle(nodes,cfg,oracle_off,wait)
            actual=np.array(samples[wait]["voltages_v"])
            max_error=max(max_error,float(max(abs(actual-expected))))
            error=abs(actual[1]-target)
            early=abs(samples[20e-6]["voltages_v"][1]-target)
            independent_early=abs(oracle(nodes,cfg,oracle_off,20e-6)[1]-target)
            assert abs(early-independent_early)<1e-9
            assert error<=tolerance and abs(expected[1]-target)<=tolerance
            assert bound["meets_deadline"] and wait<.002
            # Advance the chosen policy, not the later hypothetical baseline query.
            for n,v in zip(nodes,actual):n["v"]=float(v)
            oracle_state=expected
            engine.ok("/v1/bundles/cs_sensor_replay/insert",dict(records=nodes))
            decisions.append(dict(source_row=row,raw_count=count,channel=channel,declared_source_v=target,
                fixed_20us_error_v=early,adaptive_error_v=error,bound_s=bound["settling_bound_s"],wait_s=wait,tolerance_v=tolerance))
    assert max_error<1e-9,max_error
    early_failures=sum(r["fixed_20us_error_v"]>tolerance for r in decisions)
    assert early_failures>0
    (out/"sensor-decisions.json").write_text(json.dumps(decisions,indent=2),encoding="utf-8")
    return dict(source_url=DATA_URL,source_sha256=DATA_SHA,measured_bundle="cs_measured_counts",measured_rows=len(counts),simulated_acquisitions=len(decisions),
        early_sampling_failures=early_failures,adaptive_failures=0,max_oracle_error_v=max_error,
        fixed_2ms_total_wait_s=.002*len(decisions),adaptive_total_wait_s=sum(r["wait_s"] for r in decisions),
        max_adaptive_error_v=max(r["adaptive_error_v"] for r in decisions),tolerance_v=tolerance,
        hardware_gate="OPEN: raw counts lack calibrated volts and timestamps; circuit is explicitly hypothetical")

def main():
    parser=argparse.ArgumentParser();parser.add_argument("--binary",type=Path,default=ROOT/"target/debug/gigi-stream.exe");parser.add_argument("--output",type=Path);parser.add_argument("--sensor-replay",action="store_true");parser.add_argument("--build",action="store_true")
    args=parser.parse_args();out=args.output or Path(tempfile.mkdtemp(prefix="gigi-conduct-"));out.mkdir(parents=True,exist_ok=True)
    def source_hashes():
        paths=sorted(p for p in (ROOT/"src").rglob("*") if p.is_file())+[ROOT/"Cargo.toml",ROOT/"Cargo.lock",ROOT/"openapi.json"]
        return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    before=source_hashes()
    probe_digest=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    build_command=None
    if args.build:
        assert args.binary.resolve()==(ROOT/"target/debug/gigi-stream.exe").resolve(),"--build uses the standard debug binary"
        build_command=["cargo","build","--features","kahler","--bin","gigi-stream","--target-dir",str(ROOT/"target")]
        with (out/"build.log").open("wb") as log:
            subprocess.run(build_command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,check=True)
        assert before==source_hashes(),"source changed during build; refuse mixed provenance"
    report=dict(status="RUNNING",binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),head=subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip())
    report.update(build_command=build_command,rustc=subprocess.check_output(["rustc","-Vv"],text=True),binary_mtime_ns=args.binary.stat().st_mtime_ns)
    try:
        with Engine(args.binary,out) as engine:
            report["math"]=math_gates(engine)
            report["network_sizes"]=network_sizes(engine)
            report["wire"]=wire_gates(engine)
            if args.sensor_replay:report["sensor_replay"]=sensor_replay(engine,out)
        report["status"]="PASS_SOFTWARE_MODEL_GATES_NOT_HARDWARE_QUALIFIED"
        assert before==source_hashes(),"source changed during the live run; refuse mixed provenance"
        assert probe_digest==hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),"probe changed during execution"
    except BaseException as error:
        report["status"]="FAIL";report["error"]=repr(error);raise
    finally:
        report["source_sha256"]=before
        report["probe_sha256"]=probe_digest
        (out/"report.json").write_text(json.dumps(report,indent=2),encoding="utf-8")
        print(json.dumps({k:v for k,v in report.items() if k!="source_sha256"},indent=2));print(f"Evidence: {out}")

if __name__=="__main__":main()
