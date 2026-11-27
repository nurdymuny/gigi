"""Focused geometry and document checks for the introductory doublet figure."""
import argparse
from copy import deepcopy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import unittest

import numpy as np
import sympy as sp

from build_lowest_note_doublet import doublet, figure_data, rotation
from check_lowest_note_format import amended_h7_baseline, mathematics, statements
from lowest_note_wick_amendment import amend_wick_statements, amend_wick_text

ROOT = Path(__file__).resolve().parents[1]
PAPER = ROOT/'docs/papers/ym-theory-statement'
BASELINE = '1d7ce3e77cc5df8782b5080d93ad25225cfdcc63'


def validate_data(data):
    expected = np.array([[[0,0,0],[0,1,0],[0,0,-1]],[[0,0,0],[0,0,1],[0,1,0]]])
    np.testing.assert_array_equal(data['matrices'],expected)
    np.testing.assert_array_equal(data['images'],expected)
    np.testing.assert_array_equal([r['theta_degrees'] for r in data['orbit']],[0,45,90,135,180])
    directions = ((1,0),(0,1),(-1,0),(0,-1),(1,0))
    for state,(a,b) in zip(data['orbit'],directions):
        np.testing.assert_allclose(state['coefficients'],[a,b],atol=1e-14,rtol=0)
        np.testing.assert_allclose(state['matrix'],a*expected[0]+b*expected[1],atol=1e-14,rtol=0)


class Geometry(unittest.TestCase):
    def test_actual_matrices_maps_and_plotted_orbit(self):
        validate_data(figure_data())

    def test_charge_two_with_declared_orientation(self):
        theta = sp.symbols('theta',real=True)
        r = sp.diag(1,1,1)
        r[1:3,1:3] = sp.Matrix([[sp.cos(theta),-sp.sin(theta)],[sp.sin(theta),sp.cos(theta)]])
        a,b = [sp.Matrix(d.astype(int)) for d in doublet()]
        self.assertEqual(sp.simplify(r*a*r.T-sp.cos(2*theta)*a-sp.sin(2*theta)*b),sp.zeros(3))
        self.assertEqual(sp.simplify(r*b*r.T+sp.sin(2*theta)*a-sp.cos(2*theta)*b),sp.zeros(3))
        # The complex combinations carry opposite characters in this active convention.
        self.assertEqual(sp.simplify(r*(a+sp.I*b)*r.T-(sp.cos(2*theta)-sp.I*sp.sin(2*theta))*(a+sp.I*b)),sp.zeros(3))

    def test_norm_orthogonality_and_omitted_direction(self):
        a,b = doublet()
        self.assertEqual(np.sum(a*a),2)
        self.assertEqual(np.sum(b*b),2)
        self.assertEqual(np.sum(a*b),0)
        for d in (a,b):
            np.testing.assert_array_equal(d,d.T)
            np.testing.assert_array_equal(d.T@np.array([1,0,0]),[0,0,0])
            self.assertEqual(np.trace(d),0)

    def test_quarter_turn_is_negative_not_a_return(self):
        a,_ = doublet()
        np.testing.assert_allclose(rotation(np.pi/2)@a@rotation(np.pi/2).T,-a,atol=1e-14)
        np.testing.assert_allclose(rotation(np.pi)@a@rotation(np.pi).T,a,atol=1e-14)

    def test_gauge_orthogonality_at_circle_symmetric_background(self):
        x,y = sp.symbols('x y',real=True)
        background = sp.diag(x,y,y)
        for i,j in ((0,1),(0,2),(1,2)):
            generator = sp.zeros(3); generator[i,j]=1; generator[j,i]=-1
            tangent = background*generator
            for d in doublet():
                self.assertEqual(sp.simplify(sp.trace(sp.Matrix(d.astype(int)).T*tangent)),0)

    def test_hessian_eigenvectors_at_mc_background(self):
        variables = sp.symbols('q0:9',real=True)
        q = sp.Matrix(3,3,variables)
        for t in (sp.Integer(1),sp.Integer(3),sp.Integer(7)):
            frequencies = [sp.sqrt(t),1/sp.sqrt(t),1/sp.sqrt(t)]
            W = sum(frequencies[k]*sum(q[k,c]**2 for c in range(3))/2 for k in range(3))-q.det()
            potential = sum(sp.diff(W,v)**2 for v in variables)/2
            point = dict(zip(variables,list(sp.diag(1/sp.sqrt(t),1,1))))
            hessian = sp.hessian(potential,variables).subs(point)
            for d in doublet():
                v = sp.Matrix(list(d.astype(int).flat))
                self.assertEqual(sp.simplify(hessian*v-4*v/t),sp.zeros(9,1))

    def test_bad_sign_missing_entry_and_false_return_are_rejected(self):
        good = figure_data()
        mutations = []
        wrong = deepcopy(good); wrong['matrices'][0][2][2]=1; mutations.append(wrong)
        wrong = deepcopy(good); wrong['matrices'][1][2][1]=0; mutations.append(wrong)
        wrong = deepcopy(good); wrong['images'][0][2]=[0,0,1]; mutations.append(wrong)
        wrong = deepcopy(good); wrong['orbit'][1]['coefficients']=[0,-1]; mutations.append(wrong)
        wrong = deepcopy(good); wrong['orbit'][2]['coefficients']=[1,0]; mutations.append(wrong)
        for i,broken in enumerate(mutations):
            with self.subTest(mutation=i),self.assertRaises(AssertionError):
                validate_data(broken)


class Document(unittest.TestCase):
    def test_introduction_precedes_the_proposal_and_explains_charge(self):
        text = (PAPER/'sections/s1-problem.tex').read_text(encoding='utf-8')
        self.assertIn('What I mean by Helike',text)
        self.assertLess(text.index('What I mean by Helike'),text.index('The proposed mechanism'))
        for word in ('not electric charge','two real','eigenvector','quadratic','fig:helike-doublet'):
            self.assertIn(word,text)
        self.assertIn(r'\includegraphics[width=\linewidth]{figures/helike-doublet.pdf}',text)

    def test_all_incoming_statements_and_math_survive(self):
        rel = 'docs/papers/ym-theory-statement/sections'
        paths = subprocess.check_output(['git','ls-tree','-r','--name-only',BASELINE,'--',rel],cwd=ROOT,text=True).splitlines()
        before = '\n'.join(subprocess.check_output(['git','show',f'{BASELINE}:{p}'],cwd=ROOT).decode('utf-8')
                           for p in paths if p.endswith('.tex'))
        after = '\n'.join(p.read_text(encoding='utf-8') for p in sorted((PAPER/'sections').glob('*.tex')))
        current = statements(after)
        self.assertEqual(current.pop('b:cubicquartic')[0], 'PROVED')
        self.assertEqual(amend_wick_statements(amended_h7_baseline(statements(before))),current)
        self.assertEqual(len(statements(after)),48)
        self.assertFalse(mathematics(amend_wick_text(before))-mathematics(after))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out',type=Path)
    parser.add_argument('--artifacts',action='store_true')
    args = parser.parse_args()
    args.out.mkdir(parents=True,exist_ok=False)
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream,verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(__import__(__name__)))
    log = stream.getvalue(); print(log,end='')
    (args.out/'checks.log').write_text(log,encoding='utf-8')
    artifacts_ok = None
    pdf_pages = None
    if args.artifacts:
        from pypdf import PdfReader
        for name in ('helike-doublet.svg','helike-doublet.json'):
            if b'\r\n' in (PAPER/'figures'/name).read_bytes():
                raise ValueError('Generated text must retain canonical LF line endings: '+name)
        data = json.loads((PAPER/'figures/helike-doublet.json').read_text())
        validate_data(data)
        for name,value in data['artifacts_sha256'].items():
            if hashlib.sha256((PAPER/'figures'/name).read_bytes()).hexdigest()!=value:
                raise ValueError('Figure artifact hash mismatch: '+name)
        actual = hashlib.sha256((ROOT/'scripts/build_lowest_note_doublet.py').read_bytes().replace(b'\r\n',b'\n')).hexdigest()
        if actual!=data['generator_sha256_lf']:
            raise ValueError('Figure generator changed after rendering')
        pdf = PdfReader(PAPER/'statement.pdf')
        pdf_pages = len(pdf.pages)
        first = pdf.pages[0].extract_text()
        second = pdf.pages[1].extract_text()
        third = pdf.pages[2].extract_text()
        if ('What I mean by Helike' not in first or 'not electric charge' not in second
                or 'Figure1:' not in ''.join(second.split()) or 'homogeneous doublet' not in second
                or 'The proposed mechanism' not in third):
            raise ValueError('Rebuilt PDF does not contain the intended opening spread')
        artifacts_ok = True
    sources = [Path(__file__),ROOT/'scripts/build_lowest_note_doublet.py',ROOT/'scripts/check_lowest_note_format.py',
               PAPER/'figures/.gitattributes',
               PAPER/'HANDOFF.md',*sorted(PAPER.rglob('*.tex'))]
    report = dict(scope='Homogeneous doublet geometry and editorial preservation, not a spectrum-floor proof',
                  source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                  baseline=BASELINE,tests=result.testsRun,failures=len(result.failures),errors=len(result.errors),
                  passed=result.wasSuccessful(),artifacts_verified=artifacts_ok,pdf_pages=pdf_pages,
                  numpy=np.__version__,sympy=sp.__version__,
                  figure_artifacts_sha256={p.name:hashlib.sha256(p.read_bytes()).hexdigest()
                                          for p in sorted((PAPER/'figures').glob('helike-doublet.*'))}
                                          if args.artifacts else {},
                  sha256_lf={p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes().replace(b'\r\n',b'\n')).hexdigest() for p in sources},
                  pdf_sha256=hashlib.sha256((PAPER/'statement.pdf').read_bytes()).hexdigest(),figure=figure_data())
    (args.out/'receipt.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    return 0 if result.wasSuccessful() else 1


if __name__=='__main__':
    raise SystemExit(main())
