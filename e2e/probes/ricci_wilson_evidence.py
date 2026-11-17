"""Independent, fail-closed RF2A verifier; scientific verdicts are separate."""
import json
import math
import os
import subprocess
from pathlib import Path
import numpy as np
from monodromy_evidence import require,sha,strict_json,checked_row
from monodromy_exact_oracle import oracle
from ricci_evidence import ROOT,manifest as ricci_manifest,setup_queries,group_query
from ricci_oracle import velocity,trajectory
from ricci_wilson_cases import audits,flows,audit_query,flow_query,REFUSALS
from ricci_wilson_oracle import (phi,check_geometry,field_curvature,energy,wilson,mapped_velocity,
                                 experiment,symbolic_certificate,operator_checks,acceleration_check,quotient_checks)

def manifest():
    result=ricci_manifest()
    p=ROOT/"docs/RICCI_WILSON_CORRESPONDENCE_SPEC.md"
    result[p.as_posix()]=sha(p)
    return result

def check_audit(item,g,n):
    require(item["path"]=="/v1/gql" and item["request"]=={"query":audit_query(g,n)},"audit query binding")
    require(item["status"]==200,"audit HTTP")
    rows=item["response"].get("rows",[])
    require(len(rows)==1,"actual audit row required")
    r=rows[0]; g=np.array(g,dtype=float)
    a=check_geometry(g); x=np.diag(a)
    p=mapped_velocity(g,n); w=wilson(g,x)
    for key,expected in [("metric",g),("connection",x),("ricci_velocity",velocity(g,n)),
            ("connection_curvature",np.diag(field_curvature(a))),
            ("mapped_ricci_velocity",p),("yang_mills_velocity",w)]:
        require(type(r[key]) is list and len(r[key])==3,"vector dimensions")
        require(all(type(v) in (int,float) and math.isfinite(v) for v in r[key]),"numeric vector required")
        np.testing.assert_allclose(r[key],expected,atol=3e-11,rtol=3e-13,err_msg=key)
    # Recompute diagnostics from the RAW returned operators, not from a zero identity.
    returned_p=np.array(r["mapped_ricci_velocity"]); returned_w=np.array(r["yang_mills_velocity"])
    difference=returned_p-returned_w
    np.testing.assert_array_equal(r["residual"],difference)
    norm=lambda z:math.sqrt(float(np.sum(z*z/g)))
    den=max(norm(returned_p),norm(returned_w))
    for key,expected in [("absolute_residual",norm(difference)),
        ("relative_residual",norm(difference)/den if den else 0.),
        ("mapped_velocity_norm",norm(returned_p)),("yang_mills_velocity_norm",norm(returned_w)),
        ("energy_density",float(energy(g,a)))]:
        require(type(r[key]) in (int,float) and math.isfinite(r[key]),"numeric diagnostic required")
        np.testing.assert_allclose(r[key],expected,rtol=1e-12,atol=1e-25,err_msg=key)
    require(r["relative_residual"]<2e-11,"H1 residual exceeds numerical allowance")
    require(r["scope"]=="homogeneous_levi_civita_instantaneous","scope")
    require(r["mapping"]=="levi_civita_positive_orthonormal_frame","mapping")
    require(r["gradient"]=="negative_YM_with_instantaneous_g_held_fixed","gradient")
    require(r["numerical_policy"]=="ricci_wilson_v1","policy")
    require(r["algebra_convention"]=="so3_J_cross_bracket_plus","algebra")
    require(r["norm_convention"]=="L2_per_unit_volume","norm")
    require(r["manifold_model"]=="SU2_left_invariant_diagonal_cover","manifold")
    require(r["normalized"] is n and type(r["time_scale"]) in (int,float)
            and r["time_scale"]==1.,"fixed numerical time convention")
    for key in ["time_scale_fitted","lattice_correspondence_tested","fixed_background_equivalence_tested","topology_inferred"]:
        require(r[key] is False,f"unsupported claim {key}")
    return r["relative_residual"]

def verify(path):
    path=Path(path); receipt=path.with_suffix(".PASS.json")
    receipt.unlink(missing_ok=True)
    d=strict_json(path.read_text(encoding="utf-8"))
    require(d["schema"]=="ricci_wilson_live_v1","schema")
    require(d["sources"]==manifest(),"changed or incomplete sources")
    require(sha(d["binary"]["path"])==d["binary"]["sha256"],"binary mismatch")
    require(d["head"]==subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),"revision")
    require(d["symbolic"]==symbolic_certificate(),"symbolic certificate mismatch")
    checks=operator_checks()
    require(d["operator_checks"]==checks,"independent operator checks")
    require(d["acceleration_check"]==acceleration_check(),"independent acceleration check")
    require(d["quotient_checks"]==quotient_checks(),"quotient action and L2 mass scaling")
    records={}
    for item in d["requests"]:
        name=item["label"]
        require(name not in records,"duplicate request")
        require(strict_json(item["raw_response"])==item["response"],"raw/parsed mismatch")
        records[name]=item
    expected=set(audits())|set(REFUSALS)|{"lattice","group_create","group_insert","group_load","group120","group1"}
    for name in flows():
        expected.add("flow_"+name)
        expected.update(f"checkpoint_{name}_{i}" for i in range(0,257,32))
    require(set(records)==expected,"missing or additional cases")
    residuals=[check_audit(records[name],*case) for name,case in audits().items()]
    summaries={}
    require(set(d["reference_experiments"])==set(flows()),"missing reference experiments")
    for name,(g,n) in flows().items():
        item=records["flow_"+name]
        require(item["status"]==200 and item["request"]=={"query":flow_query(g,n)},"flow binding")
        rows=item["response"]["rows"]
        require(len(rows)==257,"full Ricci trajectory")
        metrics=np.array([[r[k] for k in ["a","b","c"]] for r in rows])
        np.testing.assert_allclose(metrics,trajectory(g,.02,256,n),rtol=5e-10,atol=5e-10)
        np.testing.assert_allclose([r["t"] for r in rows],np.linspace(0,.02,257),rtol=0,atol=1e-15)
        for i in range(0,257,32):
            residuals.append(check_audit(records[f"checkpoint_{name}_{i}"],metrics[i],n))
        loose=experiment(g,.02,n,rtol=2e-10)
        tight=experiment(g,.02,n,rtol=2e-12)
        saved=d["reference_experiments"][name]
        for key in tight:
            np.testing.assert_allclose(saved[key],tight[key],rtol=1e-12,atol=1e-13,err_msg=f"reference tampering {name}.{key}")
        mapped=np.array([phi(q) for q in metrics[::4]])
        moving_error=float(np.max(np.abs(mapped-np.array(tight["moving_background"]))))
        fixed_gap=float(np.max(np.abs(mapped-np.array(tight["fixed_background"]))))
        require(moving_error<2e-8,"H2 moving-background tracking")
        refinement=float(np.max(np.abs(np.array(tight["fixed_background"])-np.array(loose["fixed_background"]))))
        require(refinement<2e-8,"reference tolerance refinement")
        if name.startswith("round_"):
            require(fixed_gap<1e-12,"round critical control")
        else:
            require(fixed_gap>1e-4 and fixed_gap>1000*max(refinement,1e-12),"planted H3 fixed-background separation")
        fixed_energy=[float(energy(np.array(g),np.diag(x))) for x in tight["fixed_background"]]
        require(max(np.diff(fixed_energy))<1e-9,"fixed-background negative gradient energy")
        summaries[name]={"moving_background_error_against_native_Ricci":moving_error,
            "fixed_background_gap_against_native_Ricci":fixed_gap,"fixed_reference_refinement":refinement,
            "fixed_end_gap":tight["fixed_end_gap"],"fixed_energy_initial":fixed_energy[0],"fixed_energy_final":fixed_energy[-1]}
    for name,(query,fragment) in REFUSALS.items():
        item=records[name]
        require(item["request"]=={"query":query} and item["status"]>=400,"refusal binding")
        require(fragment in item["response"].get("error","") and "rows" not in item["response"],"explicit refusal")
    exact=oracle()
    require(d["group_oracle"]==json.loads(json.dumps(exact)),"exact group certificate")
    phi5=(1+math.sqrt(5))/2
    qs=[[.5]*4,[phi5/2,.5,(phi5-1)/2,0.],[1.,0.,0.,0.],[1.,0.,0.,0.]]
    expected_records=[dict(config_id=0,edge_id=e,vertex_a=2*e,vertex_b=2*e+1,**{f"q{i}":q[i] for i in range(4)}) for e,q in enumerate(qs)]
    require(records["group_insert"]["request"]=={"records":expected_records},"deck input binding")
    require(records["group_insert"]["path"]=="/v1/bundles/rf_group_data/insert","deck insertion target")
    for name,query in setup_queries().items():require(records[name]["request"]=={"query":query},"deck setup")
    for name in ["lattice","group_create","group_insert","group_load"]: require(records[name]["status"]==200,"deck setup HTTP")
    for name,edge,order in [("group120","0,1",120),("group1","2",1)]:
        require(records[name]["request"]=={"query":group_query(edge)},"closure query")
        r=checked_row(records[name],name)
        require(r["group_order"]==order and r["commutator_subgroup_order"]==order,"closure order")
        require(r["table_verified"] is True and r["associativity_checks"]==order**3,"closure table")
    require(records["sphere"]["response"]==records["poincare"]["response"],"same local quotient operators")
    result={"status":"PASS","scope":"RF2A local mathematical evidence, not release approval",
        "evidence_sha256":sha(path),"native_audits":len(residuals),"native_trajectories":len(flows()),
        "max_relative_operator_residual":max(residuals),"refusals":len(REFUSALS),
        "H1":"exact symbolic identity in the specified homogeneous family, numerically checked",
        "H2":"moving-background tracking supported; metric evolution is supplied",
        "H3":"fixed-background trajectory equivalence rejected by nonround controls",
        "lattice_correspondence_tested":False,"experiments":summaries}
    temp=receipt.with_suffix(".tmp")
    temp.write_text(json.dumps(result,indent=2,allow_nan=False),encoding="utf-8")
    os.replace(temp,receipt)
    return result
