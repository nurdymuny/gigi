"""Run real native queries against a fresh, isolated server; never supply answers."""
import argparse
import http.client
import json
import math
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import scipy
import numpy
import sys
from monodromy_evidence import sha, strict_json
from monodromy_exact_oracle import oracle
from ricci_cases import fixtures, query, REFUSALS
from ricci_oracle import self_check
from ricci_evidence import ROOT, manifest, verify, setup_queries, group_query


def run(binary,output):
    output.mkdir(parents=True,exist_ok=True)
    path = output/"live.json"
    path.with_suffix(".PASS.json").unlink(missing_ok=True)
    data = {"schema":"ricci_live_v1","head":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
            "sources":manifest(),"binary":{"path":str(binary),"sha256":sha(binary)},
            "python_version":sys.version,"numpy_version":numpy.__version__,
            "scipy_version":scipy.__version__,"oracle_checks":self_check(),"group_oracle":oracle(),"requests":[]}
    with socket.socket() as s:
        s.bind(("127.0.0.1",0))
        port = s.getsockname()[1]
    def call(name,body,path="/v1/gql"):
        conn = http.client.HTTPConnection("127.0.0.1",port,timeout=60)
        try:
            conn.request("POST",path,json.dumps(body,allow_nan=False),{"Content-Type":"application/json","Accept":"application/json"})
            resp = conn.getresponse()
            raw = resp.read().decode("utf-8")
            data["requests"].append({"label":name,"path":path,"request":body,"status":resp.status,
                                     "raw_response":raw,"response":strict_json(raw)})
        finally:
            conn.close()
    with tempfile.TemporaryDirectory(prefix="gigi-ricci-") as scratch, (output/"server.log").open("w") as log:
        env = os.environ.copy()
        for key in ["GIGI_JWT_SECRET","GIGI_API_KEY"]: env.pop(key,None)
        env.update(PORT=str(port),GIGI_DATA_DIR=scratch,RUST_LOG="warn")
        process = subprocess.Popen([str(binary)],cwd=ROOT,env=env,stdout=log,stderr=log,
                                   creationflags=getattr(subprocess,"CREATE_NO_WINDOW",0))
        try:
            for _ in range(300):
                if process.poll() is not None: raise RuntimeError("private server exited")
                try:
                    with socket.create_connection(("127.0.0.1",port),timeout=1): break
                except OSError: time.sleep(.1)
            else: raise RuntimeError("private server did not start")
            for name,case in fixtures().items(): call(name,{"query":query(case)})
            for name,(q,_) in REFUSALS.items(): call(name,{"query":q})
            setup = setup_queries()
            for name in ["lattice","group_create"]: call(name,{"query":setup[name]})
            phi = (1+math.sqrt(5))/2
            qs = [[.5]*4,[phi/2,.5,(phi-1)/2,0.],[1.,0.,0.,0.],[1.,0.,0.,0.]]
            records = [dict(config_id=0,edge_id=e,vertex_a=2*e,vertex_b=2*e+1,
                            **{f"q{i}":q[i] for i in range(4)}) for e,q in enumerate(qs)]
            call("group_insert",{"records":records},"/v1/bundles/rf_group_data/insert")
            call("group_load",{"query":setup["group_load"]})
            call("group120",{"query":group_query("0,1")})
            call("group1",{"query":group_query("2")})
        finally:
            process.terminate()
            try: process.wait(timeout=15)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            path.write_text(json.dumps(data,indent=2,allow_nan=False),encoding="utf-8")
    result = verify(path)
    print(json.dumps({k:v for k,v in result.items() if k not in ["errors","end_states"]},indent=2))
    for name in ["round_shrink","rounding","negative_scalar","triaxial"]:
        print(name, result["end_states"][name])


if __name__ == "__main__":
    p = argparse.ArgumentParser()
    p.add_argument("--binary",type=Path,required=True)
    p.add_argument("--output",type=Path,required=True)
    a = p.parse_args()
    run(a.binary.resolve(),a.output.resolve())
