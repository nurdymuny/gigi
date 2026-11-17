"""Fail-closed evidence verifier. A PASS here covers live math, not release regression."""
import json
import math
import os
from pathlib import Path
import numpy as np
from monodromy_evidence import sha, strict_json, require, checked_row
from monodromy_exact_oracle import oracle
from ricci_cases import fixtures, query, REFUSALS
from ricci_oracle import curvature, self_check, trajectory

ROOT = Path(__file__).resolve().parents[2]


def manifest():
    paths = list((ROOT/"src").rglob("*.rs")) + list((ROOT/"tests").rglob("*.rs"))
    paths += [ROOT/"Cargo.toml", ROOT/"Cargo.lock", ROOT/"GQL_REFERENCE.md", ROOT/"docs/RICCI_FLOW_SU2_SPEC.md"]
    for pattern in ["*ricci*.py","monodromy_evidence.py","monodromy_exact_oracle.py"]:
        paths += list((ROOT/"e2e/probes").glob(pattern))
    paths += list((ROOT/"scripts").glob("ricci*.py"))
    return {p.as_posix(): sha(p) for p in sorted(set(paths))}


def verify(path):
    path = Path(path)
    receipt = path.with_suffix(".PASS.json")
    receipt.unlink(missing_ok=True)
    data = strict_json(path.read_text(encoding="utf-8"))
    require(data["schema"] == "ricci_live_v1", "schema")
    require(data["sources"] == manifest(), "source manifest changed or incomplete")
    require(sha(data["binary"]["path"]) == data["binary"]["sha256"], "binary mismatch")
    require(len(data["head"]) == 40, "source revision missing")
    require(data["oracle_checks"] == self_check(), "oracle checks")
    expected = fixtures()
    by_name = {}
    for item in data["requests"]:
        name = item["label"]
        require(name not in by_name, f"duplicate case {name}")
        require(strict_json(item["raw_response"]) == item["response"], f"wire mismatch {name}")
        by_name[name] = item
    require(set(by_name) == set(expected)|set(REFUSALS)|{"lattice","group_create","group_insert","group_load","group120","group1"}, "missing or unexpected cases")
    errors = {}
    end_states = {}
    for name,case in expected.items():
        item = by_name[name]
        require(item["request"] == {"query":query(case)}, f"query mismatch {name}")
        require(item["path"] == "/v1/gql" and item["status"] == 200, f"HTTP {name}")
        rows = item["response"]["rows"]
        g,t,n,normalized,tol = case
        require(len(rows) == n+1, f"partial trajectory {name}")
        actual = np.array([[r[k] for k in ["a","b","c"]] for r in rows])
        require(np.isfinite(actual).all() and (actual>0).all(), f"positive trajectory {name}")
        np.testing.assert_allclose(actual[0],g,rtol=0,atol=1e-14)
        reference = trajectory(g,t,n,normalized)
        error = float(np.max(np.abs(actual-reference))/max(g))
        errors[name] = error
        # This checks accuracy independently, not whether GIGI reported a small estimate.
        bound = 1e-6 if name.startswith("refine_") else 5e-9
        require(error < bound, f"independent trajectory {name}: {error} >= {bound}")
        for i,r in enumerate(rows):
            for key in ["t","a","b","c","ricci_a","ricci_b","ricci_c","lambda_a","lambda_b","lambda_c",
                        "scalar_curvature","sectional_ab","sectional_bc","sectional_ca","volume_cover",
                        "anisotropy","estimated_relative_error","max_estimated_relative_error","tolerance"]:
                require(type(r[key]) in (int,float) and math.isfinite(r[key]), f"non-numeric output {name}.{key}")
            require(abs(r["t"]-t*i/n)<1e-13*max(1,t), f"wrong time {name}")
            metric = actual[i]
            ric, scalar, sect = curvature(metric)
            np.testing.assert_allclose([r[k] for k in ["ricci_a","ricci_b","ricci_c"]],ric,rtol=2e-12,atol=1e-11)
            np.testing.assert_allclose([r[k] for k in ["sectional_ab","sectional_bc","sectional_ca"]],sect,rtol=2e-12,atol=1e-11)
            np.testing.assert_allclose([r[k] for k in ["lambda_a","lambda_b","lambda_c"]],ric/metric,rtol=2e-12,atol=1e-11)
            require(abs(r["scalar_curvature"]-scalar)<1e-10, f"scalar curvature {name}")
            np.testing.assert_allclose(r["volume_cover"],2*math.pi**2*np.sqrt(np.prod(metric)),rtol=2e-12)
            np.testing.assert_allclose(r["anisotropy"],max(metric)/min(metric),rtol=2e-12)
            require(r["normalized"] is normalized, f"normalization {name}")
            require(r["tolerance"] == tol and 0 <= r["estimated_relative_error"] <= r["max_estimated_relative_error"] <= tol, f"error estimate {name}")
            require(r["rhs_evaluations"] == 12*n and r["coarse_steps"] == n, f"bounded work {name}")
            for flag in ["surgery_performed","topology_inferred","error_bound_certified"]:
                require(r[flag] is False, f"unsupported claim: {flag}")
            require(r["numerical_policy"] == "su2_ricci_v1" and r["method"] == "rk4_step_doubling", "numerical policy")
            require(r["manifold_model"] == "SU2_left_invariant_diagonal_cover", "manifold scope")
            require(r["bracket_convention"] == "[e1,e2]=2e3_cyclic", "bracket convention")
            require(r["equation"] == ("dg/dt=-2Ric+(2/3)Rg" if normalized else "dg/dt=-2Ric"), "flow equation")
        require(all(r["max_estimated_relative_error"] == max(s["estimated_relative_error"] for s in rows)
                    for r in rows), f"inconsistent maximum error estimate {name}")
        if normalized:
            require(float(np.max(np.abs(np.prod(actual,axis=1)/np.prod(g)-1))) < 2e-6, f"volume conservation {name}")
        if name == "round_shrink":
            np.testing.assert_allclose(actual, np.repeat((1-4*np.linspace(0,t,n+1))[:,None],3,axis=1),atol=1e-13,rtol=0)
        end_states[name] = {k:rows[-1][k] for k in ["a","b","c","scalar_curvature","anisotropy","volume_cover"]}
    ratios = [errors["refine_16"]/errors["refine_32"], errors["refine_32"]/errors["refine_64"]]
    require(all(12 < r < 22 for r in ratios), f"fourth-order convergence {ratios}")
    require(end_states["rounding"]["anisotropy"] < 1.00001, "normalized rounding")
    require(by_name["negative_scalar"]["response"]["rows"][0]["scalar_curvature"] == -2., "negative curvature clipped")
    for name,(q,fragment) in REFUSALS.items():
        item = by_name[name]
        require(item["request"] == {"query":q} and item["path"] == "/v1/gql", f"refusal query {name}")
        require(item["status"] >= 400 and fragment in item["response"].get("error",""), f"refusal {name}")
        require("rows" not in item["response"], f"partial refusal {name}")
    exact = oracle()
    require(data["group_oracle"] == json.loads(json.dumps(exact)), "exact group certificate")
    require(exact["group_order"] == 120 and exact["commutator_subgroup_order"] == 120, "exact group failed")
    # Bind the native generators to the independently certified quotient construction.
    phi = (1+math.sqrt(5))/2
    qs = [[.5]*4, [phi/2,.5,(phi-1)/2,0.], [1.,0.,0.,0.], [1.,0.,0.,0.]]
    records = [dict(config_id=0,edge_id=e,vertex_a=2*e,vertex_b=2*e+1,
                    **{f"q{i}":q[i] for i in range(4)}) for e,q in enumerate(qs)]
    require(by_name["group_insert"]["request"] == {"records":records}, "group generator input")
    require(by_name["group_insert"]["path"] == "/v1/bundles/rf_group_data/insert", "group insert target")
    for name,q in setup_queries().items():
        require(by_name[name]["request"] == {"query":q}, f"group setup {name}")
    for name in ["lattice","group_create","group_insert","group_load"]:
        require(by_name[name]["status"] == 200, f"group setup HTTP {name}")
    for name,edges,order in [("group120","0,1",120),("group1","2",1)]:
        require(by_name[name]["request"] == {"query":group_query(edges)}, "group query mismatch")
        r = checked_row(by_name[name],name)
        require(r["group_order"] == order and r["commutator_subgroup_order"] == order, f"group closure {name}")
        require(r["table_verified"] is True and r["associativity_checks"] == order**3, "group table verification")
    result = {"status":"PASS","scope":"RF1 live mathematical evidence only",
              "evidence_sha256":sha(path),"trajectory_cases":len(expected),"refusals":len(REFUSALS),
              "max_nonrefinement_error":max(v for k,v in errors.items() if not k.startswith("refine_")),
              "refinement_ratios":ratios,"errors":errors,"end_states":end_states,
              "group_orders":[120,1],"topology_recognition_claimed":False,
              "wilson_ricci_equivalence_claimed":False}
    temp = receipt.with_suffix(".tmp")
    temp.write_text(json.dumps(result,indent=2,allow_nan=False),encoding="utf-8")
    os.replace(temp,receipt)
    return result


def setup_queries():
    return {
        "lattice":"LATTICE rf_lat VERTICES 8 EDGES ((0,1),(2,3),(4,5),(6,7)) FACES ();",
        "group_create":"CREATE BUNDLE rf_group_data (config_id INT BASE, edge_id INT BASE, vertex_a INT BASE, vertex_b INT BASE, q0 NUMERIC FIBER, q1 NUMERIC FIBER, q2 NUMERIC FIBER, q3 NUMERIC FIBER);",
        "group_load":"GAUGE_FIELD rf_group GROUP SU(2) INIT FROM BUNDLE rf_group_data ON LATTICE rf_lat;",
    }


def group_query(edges):
    return f"MONODROMY_GROUP_ORDER rf_group FROM EDGES ({edges}) COMMUTATOR CAP 120 TOLERANCE 0.00000001;"
