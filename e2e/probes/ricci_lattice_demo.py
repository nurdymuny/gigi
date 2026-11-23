"""RF2B native-kernel evidence. Never calls transport convergence flow convergence."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import numpy as np
from scipy.linalg import expm,logm
from ricci_lattice_mesh import mesh,validate,deck_check,volume,plus
from ricci_wilson_oracle import connection_matrices,axial,field_curvature,wilson
from monodromy_evidence import sha
from ricci_lattice_forms import masses

ROOT=Path(__file__).resolve().parents[2]
SIGMA=np.array([[[0,1],[1,0]],[[0,-1j],[1j,0]],[[1,0],[0,-1]]],dtype=complex)


def matrix(q):
    q=np.asarray(q)
    return q[...,0,None,None]*np.eye(2)+1j*np.einsum('...i,ijk->...jk',q[...,1:],SIGMA)


def matrix_link(g,p,q):
    # Base-space plus quaternions use q0 I - i q.sigma, unlike stored links.
    p,q=np.asarray(p),np.asarray(q)
    a=matrix(np.r_[p[0],-p[1:]]); b=matrix(np.r_[q[0],-q[1:]])
    logarithm=logm(a.conj().T@b)
    xi=np.array([(0.5j*np.trace(s@logarithm)).real for s in SIGMA])
    x=np.diag(axial(connection_matrices(np.array(g))))
    return expm(-0.5j*np.einsum('i,ijk->jk',x*xi,SIGMA))


def verify_native(m,g,result):
    assert result["schema"]=="rf2b_native_probe_v1"
    assert result["metric_mass_present"] is False
    assert result["lattice_operator_convergence_tested"] is False
    assert result["scope"]=="native_transport_and_legacy_force_only"
    assert result["metric"]==list(g) and result["beta"]==2.5
    for key in ["vertices","edges","faces"]:assert result[key]==len(m[key])
    links=np.array(result["links"]); hol=np.array(result["face_holonomies"]); force=np.array(result["legacy_forces"])
    assert links.shape==(len(m["edges"]),4) and hol.shape==(len(m["faces"]),4) and force.shape==links.shape
    assert np.isfinite(links).all() and np.isfinite(hol).all() and np.isfinite(force).all()
    np.testing.assert_allclose(np.linalg.norm(links,axis=1),1,atol=1e-13,rtol=0)
    chosen=np.unique(np.linspace(0,len(links)-1,min(96,len(links)),dtype=int))
    for i in chosen:
        a,b=m["edges"][i]
        np.testing.assert_allclose(matrix(links[i]),matrix_link(g,m["vertices"][a],m["vertices"][b]),atol=2e-12,rtol=2e-12)
    index={tuple(e):i for i,e in enumerate(m["edges"])}
    faces=m["faces"]
    cycle=np.column_stack([[index[tuple(row)] for row in faces[:,[a,b]]] for a,b in [(0,1),(1,2),(0,2)]])
    matrices=matrix(links)
    products=matrices[cycle[:,0]]@matrices[cycle[:,1]]@matrices[cycle[:,2]].conj().transpose(0,2,1)
    np.testing.assert_allclose(matrix(hol),products,atol=3e-13,rtol=3e-13)
    potential=.625*np.sum(1-np.trace(products,axis1=1,axis2=2).real/2)
    np.testing.assert_allclose(result["legacy_potential"],potential,atol=1e-10,rtol=2e-12)
    max_derivative_error=0.
    for eid in chosen[::8]:
        adjacent=np.where(np.any(cycle==eid,axis=1))[0]
        for axis in range(3):
            h=1e-5; values=[]
            for sign in [1,-1]:
                modified=matrices[eid]@expm(sign*h*1j*SIGMA[axis])
                products=[]
                for face_id in adjacent:
                    out=np.eye(2,dtype=complex)
                    for position,edge in enumerate(cycle[face_id]):
                        u=modified if edge==eid else matrices[edge]
                        out=out@(u.conj().T if position==2 else u)
                    products.append(np.trace(out).real/2)
                values.append(.625*np.sum(1-np.array(products)))
            derivative=(values[0]-values[1])/(2*h)
            max_derivative_error=max(max_derivative_error,abs(derivative+2*force[eid,axis+1]))
    assert max_derivative_error<3e-9,max_derivative_error
    expected_samples={(i,n) for i in [0,len(m["tetrahedra"])//3,2*len(m["tetrahedra"])//3,len(m["tetrahedra"])-1] for n in [2,4,6]}
    assert {(r["cell"],r["order"]) for r in result["cell_mass_samples"]}==expected_samples
    for sample in result["cell_mass_samples"]:
        reference=masses(m["vertices"][m["tetrahedra"][sample["cell"]]],g,sample["order"])
        for key in ["edge_mass","face_mass","volume"]:
            np.testing.assert_allclose(sample[key],reference[key],atol=2e-11,rtol=3e-12)
        for key in ["edge_mass","face_mass"]:
            assert np.linalg.eigvalsh(sample[key]).min()>0
    p=m["vertices"][faces[:,0]]; conjugate=p.copy();conjugate[:,1:]*=-1
    tangent=[]
    for j in [1,2]:
        delta=plus(conjugate,m["vertices"][faces[:,j]])
        length=np.linalg.norm(delta[:,1:],axis=1)
        tangent.append(delta[:,1:]*(np.arctan2(length,delta[:,0])/length)[:,None])
    area=np.cross(tangent[0],tangent[1])/2
    f=np.diag(field_curvature(axial(connection_matrices(np.array(g)))))
    expected=area*f
    qlength=np.linalg.norm(hol[:,1:],axis=1)
    factor=np.divide(-2*np.arctan2(qlength,hol[:,0]),qlength,out=np.full_like(qlength,-2.),where=qlength>0)
    rotation=hol[:,1:]*factor[:,None]
    error=np.linalg.norm(rotation-expected,axis=1)/np.linalg.norm(area,axis=1)
    return {"independent_matrix_links":len(chosen),"all_matrix_holonomies":len(faces),
        "native_force_derivative_max_error":float(max_derivative_error),
        "curvature_max_error":float(error.max()),"curvature_rms_error":float(np.sqrt(np.mean(error**2))),
        "legacy_potential":float(potential),"independent_full_cell_matrices":len(expected_samples)}


def run(binary,output,max_level):
    output.mkdir(parents=True,exist_ok=True)
    (output/"gate.PASS.json").unlink(missing_ok=True)
    receipt={"status":"IN_PROGRESS","scope":"RF2B prerequisites, not lattice gradient convergence",
        "head":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),
        "binary":{"path":str(binary),"sha256":sha(binary)},"levels":{},"operator_convergence_proven":False}
    files=[ROOT/"Cargo.toml",ROOT/"src/ricci_wilson.rs",ROOT/"src/gauge/ricci_link.rs",
           ROOT/"src/gauge/group_element.rs",ROOT/"src/gauge/wilson_force.rs",ROOT/"src/gauge/holonomy.rs",
           ROOT/"src/gauge/staple.rs",ROOT/"src/gauge/lie_exp.rs",ROOT/"src/gauge/ricci_whitney.rs",ROOT/"examples/ricci_lattice_probe.rs",
           ROOT/"tests/ricci_lattice.rs",ROOT/"docs/RICCI_LATTICE_CORRESPONDENCE_SPEC.md"]
    files+=list((ROOT/"e2e/probes").glob('*ricci_lattice*.py'))
    receipt["sources"]={str(p):sha(p) for p in files}
    fixtures={"round":[1.,1.,1.],"berger":[3.,1.,1.],"triaxial":[.5,1.,2.],"negative":[5.,1.,1.]}
    try:
        for level in range(max_level+1):
            m=mesh(level); info={"mesh":validate(m),"quotient":deck_check(m),"volume":volume(m),"cases":{}}
            receipt["levels"][str(level)]=info
            print("level",level,info["mesh"],"volume",info["volume"],flush=True)
            for name,g in fixtures.items():
                inp=output/f"level{level}_{name}.input.json"; out=output/f"level{level}_{name}.output.json"
                payload={k:m[k].tolist() for k in ["vertices","edges","faces","tetrahedra"]};payload["metric"]=g
                inp.write_text(json.dumps(payload,allow_nan=False),encoding="utf-8")
                subprocess.run([str(binary),str(inp),str(out)],check=True,timeout=300)
                result=json.loads(out.read_text())
                checks=verify_native(m,g,result)
                info["cases"][name]={"checks":checks,"input_sha256":sha(inp),"output_sha256":sha(out)}
                print(name,checks,flush=True)
            if level==0:
                original=json.loads((output/"level0_berger.output.json").read_text())
                payload["metric"]=[21.,7.,7.]
                inp=output/"scaled.input.json";out=output/"scaled.output.json"
                inp.write_text(json.dumps(payload),encoding="utf-8")
                subprocess.run([str(binary),str(inp),str(out)],check=True,timeout=300)
                scaled=json.loads(out.read_text());verify_native(m,[21.,7.,7.],scaled)
                force_delta=float(np.max(np.abs(np.array(original["legacy_forces"])-np.array(scaled["legacy_forces"]))))
                assert force_delta<1e-13
                a=np.diag(axial(connection_matrices(np.array([3.,1.,1.]))))
                w=wilson(np.array([3.,1.,1.]),a); ws=wilson(np.array([21.,7.,7.]),a)
                np.testing.assert_allclose(ws,w/7,atol=1e-13,rtol=1e-13)
                receipt["scale_obstruction"]={"native_force_difference":force_delta,"continuum_velocity":w.tolist(),
                    "scaled_continuum_velocity":ws.tolist(),"scaled_input_sha256":sha(inp),"scaled_output_sha256":sha(out),
                    "conclusion":"legacy force needs explicit metric action and mass; no universal fixed clock repairs this"}
        for name in fixtures:
            errors=[receipt["levels"][str(i)]["cases"][name]["checks"]["curvature_max_error"] for i in range(max_level+1)]
            assert all(errors[i+1]<errors[i] for i in range(len(errors)-1)),(name,errors)
        receipt["status"]="PREREQUISITES_PASS_FULL_GATE_OPEN"
        receipt["missing"]= ["assembled covariant Whitney action and tangent metric","full discrete gradient","all-link operator convergence","space-time convergence","complete BW00-BW12 release evidence"]
    finally:
        (output/"prerequisites.json").write_text(json.dumps(receipt,indent=2,allow_nan=False),encoding="utf-8")


if __name__=="__main__":
    p=argparse.ArgumentParser();p.add_argument('--binary',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    p.add_argument('--max-level',type=int,choices=range(4),default=2)
    a=p.parse_args();run(a.binary.resolve(),a.output.resolve(),a.max_level)
