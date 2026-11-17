"""Small final mechanism-removal and stale-evidence checks; no new candidate."""
import contextlib
import io
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from ricci_final_attempt import ROOT,verify,read,write,sha
import test_ricci_minimum_energy as tests

directory=Path(sys.argv[1]);kernel=ROOT/'e2e/probes/ricci_minimum_energy.py'
source=kernel.read_text(encoding='utf-8')
assert source.count('c = j@f-z@u')==1
namespace={'__name__':'removed_minimization'}
exec(compile(source.replace('c = j@f-z@u','c = j@f'),str(kernel),'exec'),namespace)
original=tests.recover;tests.recover=namespace['recover'];stream=io.StringIO()
try:
    result=unittest.TextTestRunner(stream=stream,verbosity=2).run(
        unittest.defaultTestLoader.loadTestsFromTestCase(tests.MinimumEnergyTests))
finally:tests.recover=original
if not result.failures or result.errors:raise ValueError('removed minimization was not caught behaviorally')
report={'removed_minimization':{'failures':len(result.failures),'errors':len(result.errors),'output':stream.getvalue()},
        'attacks':{},'kernel_sha256':sha(kernel),'script_sha256':sha(Path(__file__))}
with tempfile.TemporaryDirectory(prefix='rf2g-evidence-') as temp:
    target=Path(temp);shutil.copyfile(directory/'manifest.json',target/'manifest.json')
    for attack in ['missing_level','invented_pass','nonfinite']:
        receipt=read(directory/'diagnostic.json')
        if attack=='missing_level':del receipt['cases']['level3_q6']
        elif attack=='invented_pass':receipt['gate']['pass']=True
        else:receipt['elapsed_seconds']='not-a-number'
        write(target/'diagnostic.json',receipt)
        if attack=='nonfinite':
            text=(target/'diagnostic.json').read_text().replace('"not-a-number"','NaN')
            (target/'diagnostic.json').write_text(text)
        for name in ['VERIFIED.json','gate.PASS.json']:write(target/name,{'pass':True})
        try:
            with contextlib.redirect_stdout(io.StringIO()):verify(target)
        except (ValueError,AssertionError) as error:
            if any((target/name).exists() for name in ['VERIFIED.json','gate.PASS.json']):
                raise ValueError('stale PASS survived refusal')
            report['attacks'][attack]={'rejected':True,'reason':str(error),'stale_pass_removed':True}
        else:raise ValueError('forgery accepted: '+attack)
if (directory/'CONTROLS.json').exists():raise ValueError('preserve prior control receipt')
write(directory/'CONTROLS.json',report)
print('Removed minimization rejected; three evidence attacks rejected; no stale PASS.')
