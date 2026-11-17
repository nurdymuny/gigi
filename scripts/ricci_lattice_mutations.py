"""RF2B removed-mechanism gates in an isolated source AND build cache."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT=Path(__file__).resolve().parents[1]


def run(output,target):
    output.mkdir(parents=True,exist_ok=True);receipt=output/'mutations.PASS.json';receipt.unlink(missing_ok=True)
    target=target/'ricci-lattice-mutants'
    cases=[
        ('link_half_removed','ricci_link.rs','-0.5 * half_angle.sin() / half_angle','-1.0 * half_angle.sin() / half_angle','ricci_lattice','bw02_exact_axis_transport_and_reverse'),
        ('link_orientation_reversed','ricci_link.rs','-0.5 * half_angle.sin() / half_angle','0.5 * half_angle.sin() / half_angle','ricci_lattice','bw02_exact_axis_transport_and_reverse'),
        ('metric_anisotropy_removed','ricci_whitney.rs','Matrix3::from_diagonal(&Vector3::from_row_slice(&metric))','Matrix3::identity()','ricci_whitney','bw03_metric_tensor_scaling_and_anisotropy'),
        ('origin_transport_removed','ricci_simplicial.rs','defects[3] = product(product(u[0], defects[3]), inverse(u[0]));','// removed origin transport','ricci_simplicial','bw04_full_mass_directional_derivatives_and_gauge_invariance'),
        ('face_coupling_removed','ricci_simplicial.rs','Jet::constant(2. * mass[(i, j)])','Jet::constant(if i == j {2. * mass[(i,j)]} else {0.})','ricci_simplicial','bw04_cross_face_coupling_is_part_of_energy'),
        ('action_physical_factor_removed','ricci_simplicial.rs','Jet::constant(2. * mass[(i, j)])','Jet::constant(mass[(i,j)])','ricci_simplicial','bw04_plant_one_link_and_correct_physical_factor'),
        ('tangent_physical_factor_removed','ricci_simplicial.rs','4. * mass[(e, f)]','1. * mass[(e, f)]','ricci_simplicial','bw04_tangent_mass_physical_norm_spd_and_coupling'),
        ('tangent_transport_removed','ricci_simplicial.rs','roots.map(adjoint)','[SMatrix::<f64,3,3>::identity();6]','ricci_simplicial','bw04_tangent_mass_vertex_gauge_covariance'),
        ('product_derivative_removed','ricci_simplicial.rs','self.d[i] * b.v + self.v * b.d[i]','self.d[i] * b.v','ricci_simplicial','bw04_full_mass_directional_derivatives_and_gauge_invariance'),
    ]
    results=[]
    with tempfile.TemporaryDirectory(prefix='gigi-rf2b-mutants-') as temporary:
        work=Path(temporary)
        for folder in ['src','examples','benches','dashboard']:shutil.copytree(ROOT/folder,work/folder)
        (work/'tests').mkdir()
        for name in ['ricci_lattice','ricci_whitney','ricci_simplicial']:shutil.copy2(ROOT/f'tests/{name}.rs',work/f'tests/{name}.rs')
        for name in ['Cargo.toml','Cargo.lock','build.rs','openapi.json']:
            if (ROOT/name).exists():shutil.copy2(ROOT/name,work/name)
        # copy2 preserves old mtimes. Force a pristine baseline rebuild even if
        # this isolated cache last compiled a mutant in another temporary copy.
        for folder in ['src','tests','examples']:
            for source in (work/folder).rglob('*.rs'):os.utime(source,None)
        originals={name:(work/'src/gauge'/name).read_text() for name in ['ricci_link.rs','ricci_whitney.rs','ricci_simplicial.rs']}
        def execute(name,test,filter_name=None):
            command=['cargo','test','--features','halcyon','--target-dir',str(target),'--test',test]
            if filter_name:command.append(filter_name)
            log=output/(name+'.log')
            with log.open('w') as stream:
                done=subprocess.run(command,cwd=work,stdout=stream,stderr=subprocess.STDOUT,timeout=1800,
                    env={**os.environ,'CARGO_BUILD_JOBS':'4'})
            text=log.read_text(errors='replace')
            valid=(done.returncode!=0 and filter_name+' ... FAILED' in text and 'test result: FAILED' in text and 'could not compile' not in text) if filter_name else (done.returncode==0 and 'test result: ok' in text and 'running 0 tests' not in text)
            results.append({'name':name,'command':command,'exit_code':done.returncode,'behavior_observed':valid,
                'log_sha256':hashlib.sha256(log.read_bytes()).hexdigest()})
            (output/'mutations.json').write_text(json.dumps(results,indent=2))
            if not valid:raise AssertionError(f'Invalid behavioral evidence: {name}')
            print(name+(': detected' if filter_name else ': baseline green'),flush=True)
        for test in ['ricci_lattice','ricci_whitney','ricci_simplicial']:execute('baseline_'+test,test)
        for name,file,old,new,test,filter_name in cases:
            source=work/'src/gauge'/file;original=originals[file]
            if original.count(old)!=1:raise AssertionError('anchor drift: '+name)
            source.write_text(original.replace(old,new))
            execute(name,test,filter_name)
            source.write_text(original)
    receipt.write_text(json.dumps({'status':'PASS','scope':'removed native cell mechanisms only; not correspondence',
        'detected':len(cases),'receipt_sha256':hashlib.sha256((output/'mutations.json').read_bytes()).hexdigest()},indent=2))


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--output',type=Path,required=True);p.add_argument('--target-dir',type=Path,required=True)
    a=p.parse_args();run(a.output.resolve(),a.target_dir.resolve())
