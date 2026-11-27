"""Preserve the mathematical content while checking the paper-format revision."""
import argparse
from collections import Counter
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess
import unittest

from lowest_note_wick_amendment import amend_wick_statements, amend_wick_text

ROOT = Path(__file__).resolve().parents[1]
REL = 'docs/papers/ym-theory-statement'
PAPER = ROOT / REL
BASELINE = 'df949c7a69aae59b873230aabf9ccab7e5821910'
# Statements added after the format revision, each named with its status and its numbered equations.
# Baseline statements match exactly except the two explicitly frozen evidence corrections.
ADDITIONS = {
    'b:threshold': ('COMPUTED', 2),   # 27 Nov: the kinetic-term threshold, from eq. (dressing) of the baseline
    'b:cubicquartic': ('PROVED', 0),  # 27 Nov: cited global bound plus homogeneity; H7 stays conjectural
}
H7_OLD = r'''Discreteness of the small room for general $G$ needs a cubic--quartic inequality I
have not proved.'''
H7_NEW = r'''Discreteness of the small room for general $G$ now follows from
Statement~\ref{b:cubicquartic}; the full H7 conclusion remains open.'''
ADDED_REFERENCE = r'''\bibitem{AustingWheater03} P.~Austing and J.~F.~Wheater, Adding a Myers term to the IIB matrix model, JHEP 11 (2003) 009, arXiv:hep-th/0310170v2, eq.~(2.2).'''


def amended_h7_baseline(baseline):
    """Allow exactly the authorized evidence update, never a claim/status replacement."""
    result = dict(baseline)
    status, body = result['b:h7']
    old, new = (re.sub(r'\s+', '', x) for x in (H7_OLD, H7_NEW))
    if status != 'CONJECTURE' or body.count(old) != 1:
        raise ValueError('Unexpected H7 baseline or promoted status')
    result['b:h7'] = status, body.replace(old, new)
    return result


def git(*args):
    return subprocess.check_output(['git', *args], cwd=ROOT).decode('utf-8')


def uncomment(text):
    return '\n'.join(re.split(r'(?<!\\)%', line, maxsplit=1)[0] for line in text.splitlines())


def mathematics(text):
    pattern = r'(?<!\\)\$(.*?)(?<!\\)\$|\\\[(.*?)\\\]|\\begin\{equation\}(.*?)\\end\{equation\}'
    values = []
    for match in re.finditer(pattern, uncomment(text), re.S):
        value = next(v for v in match.groups() if v is not None)
        value = re.sub(r'\\label\{[^}]+\}', '', value)
        values.append(re.sub(r'\s+', '', value))
    return Counter(values)


def statements(text):
    result = {}
    for match in re.finditer(r'\\begin\{claim\}\{([^}]+)\}(.*?)\\end\{claim\}', text, re.S):
        label = re.search(r'\\label\{(b:[^}]+)\}', match[2])
        if label is None:
            raise ValueError('Statement without a stable label')
        body = match[2][label.end():]
        body = body.replace('Boxes~', 'Statements~').replace('Box~', 'Statement~')
        body = body.replace('Table~1', r'Table~\ref{tab:register}')
        body = body.replace('Table~2', r'Table~\ref{tab:clay}')
        if label[1] in result:
            raise ValueError('Duplicate statement label')
        result[label[1]] = (match[1], re.sub(r'\s+', '', body))
    return result


class Format(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        paths = git('ls-tree', '-r', '--name-only', BASELINE, '--', REL+'/sections').splitlines()
        cls.before = '\n'.join(git('show', f'{BASELINE}:{p}') for p in paths if p.endswith('.tex'))
        cls.after = '\n'.join(p.read_text(encoding='utf-8') for p in sorted((PAPER/'sections').glob('*.tex')))
        cls.preamble = (PAPER/'statement.tex').read_text(encoding='utf-8')

    def test_every_baseline_math_fragment_survives(self):
        self.assertFalse(mathematics(amend_wick_text(self.before))-mathematics(self.after))

    def test_statement_bodies_and_statuses_are_preserved(self):
        baseline = statements(self.before)
        self.assertEqual(len(baseline), 46)
        after = statements(self.after)
        for label, (status, numbered_equations) in ADDITIONS.items():
            self.assertIn(label, after)
            actual_status, body = after.pop(label)
            self.assertEqual(actual_status, status)
            self.assertEqual(body.count(r'\begin{equation}'), numbered_equations)
        self.assertEqual(after, amend_wick_statements(amended_h7_baseline(baseline)))

    def test_wick_correction_changes_only_the_named_conditional_statement(self):
        before = amended_h7_baseline(statements(self.before))
        after = amend_wick_statements(before)
        self.assertEqual([key for key in before if before[key] != after[key]], ['b:nontrivial'])
        self.assertEqual(after['b:nontrivial'][0], 'CONDITIONAL')
        self.assertEqual(after['b:h6'], before['b:h6'])

    def test_wick_correction_rejects_unexpected_baseline_or_status(self):
        before = statements(self.before)
        for broken in (('PROVED', before['b:nontrivial'][1]),
                       ('CONDITIONAL', before['b:nontrivial'][1]+'changed')):
            with self.assertRaises(ValueError):
                amend_wick_statements(dict(before, **{'b:nontrivial': broken}))
        with self.assertRaises(ValueError):
            amend_wick_text(self.before.replace('T_O\\not\\equiv0', 'T_O=1'))

    def test_wick_correction_rejects_wrong_coefficient_and_promoted_h6(self):
        expected = amend_wick_text(self.before)
        wrong = expected.replace(r'676n', r'677n')
        self.assertTrue(mathematics(expected)-mathematics(wrong))
        current = statements(self.after)
        current['b:h6'] = ('PROVED', current['b:h6'][1])
        self.assertNotEqual(current['b:h6'], statements(expected)['b:h6'])

    def test_preservation_checks_reject_changed_math_and_status(self):
        changed_math = self.before.replace('b_0=22/3', 'b_0=23/3')
        self.assertTrue(mathematics(self.before)-mathematics(changed_math))
        changed_status = self.before.replace(r'\begin{claim}{CONJECTURE}', r'\begin{claim}{PROVED}', 1)
        self.assertNotEqual(statements(changed_status), statements(self.before))
        with self.assertRaises(ValueError):
            statements(self.before+self.before)
        promoted = statements(self.before)
        promoted['b:h7'] = ('PROVED', promoted['b:h7'][1])
        with self.assertRaises(ValueError):
            amended_h7_baseline(promoted)

    def test_abstract_is_self_contained_and_short(self):
        text = (PAPER/'sections/s0-abstract.tex').read_text(encoding='utf-8')
        match = re.search(r'\\begin\{abstract\}(.*?)\\end\{abstract\}', text, re.S)
        self.assertIsNotNone(match)
        body = match[1]
        self.assertGreaterEqual(len(body.split()), 150)
        self.assertLessEqual(len(body.split()), 250)
        self.assertNotRegex(body, r'\\(?:cite|ref|S)\b|\$|\\\[')

    def test_central_conjecture_and_maps_have_separate_homes(self):
        framing = (PAPER/'sections/s2-framing.tex').read_text(encoding='utf-8')
        conclusion = (PAPER/'sections/s10-what-it-is.tex').read_text(encoding='utf-8')
        appendix = PAPER/'sections/appendix-a-register.tex'
        self.assertIn(r'\label{b:theory}', framing)
        self.assertNotIn(r'\label{b:theory}', conclusion)
        self.assertNotIn(r'\begin{tabular}', conclusion)
        self.assertTrue(appendix.exists())
        self.assertIn(r'\label{tab:register}', appendix.read_text())
        self.assertIn(r'\label{tab:clay}', appendix.read_text())
        self.assertIn(r'\appendix', self.preamble)

    def test_readable_type_and_unboxed_statements(self):
        self.assertIn(r'\documentclass[11pt,letterpaper]', self.preamble)
        self.assertIn(r'\usepackage{newpxtext}', self.preamble)
        self.assertNotIn(r'\newtcolorbox', self.preamble)
        self.assertIn(r'\fontsize{9.5}{12}', self.preamble)
        self.assertIn(r'\begin{multicols}{2}', self.preamble)
        self.assertIn('That claim is withdrawn.', self.preamble)
        self.assertEqual(self.after.count(r'\begin{equation}'), 4+sum(n for _, n in ADDITIONS.values()))

    def test_related_work_and_evidence_are_collected(self):
        intro = (PAPER/'sections/s1-problem.tex').read_text(encoding='utf-8')
        self.assertIn('Relation to earlier work', intro)
        availability = PAPER/'sections/s11-availability.tex'
        self.assertTrue(availability.exists())
        text = availability.read_text()
        self.assertIn(BASELINE, text)
        for rel in ('research/gigi-m3-20261127/result.json',
                    'research/gigi-lattice-gauge-20261127/result.json',
                    'docs/papers/helike-quantum-probe/artifact-manifest.json'):
            self.assertIn(rel, text)
            self.assertTrue((ROOT/rel).exists())

    def test_bibliography_retains_baseline_and_one_exact_addition(self):
        before = git('show', BASELINE+':'+REL+'/refs.tex')
        after = (PAPER/'refs.tex').read_text(encoding='utf-8')
        self.assertEqual(after.count(ADDED_REFERENCE), 1)
        self.assertEqual(before.replace('\r\n','\n'), after.replace(ADDED_REFERENCE+'\n', ''))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=False)
    stream = io.StringIO()
    result = unittest.TextTestRunner(stream=stream, verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Format))
    log = stream.getvalue()
    print(log, end='')
    (args.out/'checks.log').write_text(log, encoding='utf-8')
    files = [Path(__file__), ROOT/'scripts/lowest_note_wick_amendment.py',
             *sorted((ROOT/'research/h6-wick-triangle-20261127').glob('paper-*.tex')),
             *sorted(PAPER.rglob('*.tex'))]
    evidence = dict(baseline=BASELINE, source_revision=git('rev-parse','HEAD').strip(),
                    scope='Document preservation and format only; not a physics acceptance gate',
                    tests=result.testsRun, failures=len(result.failures), errors=len(result.errors),
                    passed=result.wasSuccessful(),
                    sha256_lf={p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes().replace(b'\r\n',b'\n')).hexdigest() for p in files},
                    pdf_sha256=hashlib.sha256((PAPER/'statement.pdf').read_bytes()).hexdigest())
    (args.out/'receipt.json').write_text(json.dumps(evidence,indent=2)+'\n',encoding='utf-8')
    return 0 if result.wasSuccessful() else 1


if __name__ == '__main__':
    raise SystemExit(main())
