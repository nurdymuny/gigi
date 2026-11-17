"""Focused RF2F tests and in-memory mechanism removal; no full Cargo suite."""
import argparse
import io
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT/'e2e/probes'))
import test_ricci_variational_math as math_tests
import test_ricci_variational_adversarial as geometric_tests
from monodromy_evidence import sha


def mutate(path, before, after, testmodule, testcase, names):
    source = path.read_text(encoding='utf-8')
    if source.count(before) != 1: raise ValueError('mutation target must occur exactly once')
    namespace = {'__name__':'rf2f_mutant', '__file__':str(path)}
    exec(compile(source.replace(before, after), str(path), 'exec'), namespace)
    original = {name:getattr(testmodule, name) for name in names}
    try:
        for name in names: setattr(testmodule, name, namespace[name])
        stream = io.StringIO()
        result = unittest.TextTestRunner(stream=stream, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromTestCase(testcase))
        if not result.failures or result.errors: raise ValueError('mutation did not produce clean behavioral failure: '+stream.getvalue())
        return {'tests_run':result.testsRun,'failures':len(result.failures),'errors':len(result.errors),
                'output':stream.getvalue()}
    finally:
        for name, value in original.items(): setattr(testmodule, name, value)


def run(directory):
    directory = Path(directory); directory.mkdir(parents=True,exist_ok=True)
    target = directory/'FOCUSED_TESTS.json'
    if target.exists(): raise ValueError('preserve previous focused receipt')
    modules = ['test_ricci_variational_math','test_ricci_variational_geometry',
               'test_ricci_variational_adversarial','test_ricci_variational_evidence',
               'test_ricci_variational_stability']
    stream = io.StringIO(); suite = unittest.defaultTestLoader.loadTestsFromNames(modules)
    result = unittest.TextTestRunner(stream=stream,verbosity=2).run(suite)
    if not result.wasSuccessful() or result.testsRun != 18: raise ValueError(stream.getvalue())
    math_path = ROOT/'e2e/probes/ricci_variational_math.py'
    mutations = {}
    definitions = {
        'missing_recovery_adjoint': ('gradient = j.T@y+coupling.T@adjoint','gradient = j.T@y'),
        'missing_auxiliary_solve': ('free, receipt = solve(auxiliary, coupling@f)',
            "free = np.zeros(z.shape[1]); receipt = {'removed':True}"),
        'wrong_flux_normalization': ('normal*(1+(v == vertex))/24','normal*(1+(v == vertex))/12'),
        'removed_exact_moment_constraint': ('initial[:,:4] = inverse/area[:4]','initial[:,:4] = 0'),
    }
    for name, (before, after) in definitions.items():
        mutations[name] = mutate(math_path,before,after,math_tests,math_tests.VariationalTests,
                                ['moment_matrix','fit_patch','recovered_energy'])
    mutations['wrong_face_vertex_permutation'] = mutate(ROOT/'e2e/probes/ricci_variational_mesh.py',
        'mi.append(3*fi+ordered.index(vertex))','mi.append(3*fi+list(verts).index(vertex))',
        geometric_tests,geometric_tests.AdversarialTests,['topology'])
    sources = [math_path,ROOT/'e2e/probes/ricci_variational_mesh.py',
               ROOT/'e2e/probes/ricci_variational_assembly.py',ROOT/'e2e/probes/ricci_variational_evidence.py',
               *[ROOT/'e2e/probes'/(name+'.py') for name in modules],Path(__file__)]
    report = {'tests_passed':result.testsRun,'failures':0,'errors':0,'skips':len(result.skipped),
        'mutations_detected':mutations,'sources':{str(p.resolve()):sha(p) for p in sources},
        'output':stream.getvalue(),'scope':'focused mathematical/guard gates only; not full-flow acceptance',
        'full_regressions':'DEFERRED','release_gate':'OPEN'}
    target.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'tests_passed':result.testsRun,'mutants_detected':list(mutations)},indent=2))


if __name__ == '__main__':
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path)
    run(parser.parse_args().directory)
