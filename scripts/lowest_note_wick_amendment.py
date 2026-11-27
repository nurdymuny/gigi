"""One exact authorized publication correction; unrelated claims stay frozen."""
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT/'research/h6-wick-triangle-20261127'
BEFORE = (FIXTURES/'paper-before.tex').read_text(encoding='utf-8').strip()
AFTER = (FIXTURES/'paper-after.tex').read_text(encoding='utf-8').strip()


def amend_wick_text(text):
    pattern = r'\s+'.join(re.escape(word) for word in BEFORE.split())
    matches = list(re.finditer(pattern, text))
    if len(matches) != 1:
        raise ValueError('Expected exactly the frozen pre-correction Wick passage')
    match = matches[0]
    return text[:match.start()]+AFTER+text[match.end():]


def amend_wick_statements(baseline):
    pattern = r'\\begin\{claim\}\{([^}]+)\}.*?\\label\{b:nontrivial\}(.*?)\\end\{claim\}'
    old, new = (re.search(pattern, text, re.S) for text in (BEFORE, AFTER))
    expected = old[1], re.sub(r'\s+', '', old[2])
    if baseline.get('b:nontrivial') != expected or old[1] != 'CONDITIONAL' or new[1] != 'CONDITIONAL':
        raise ValueError('Unexpected non-triviality baseline or promoted status')
    result = dict(baseline)
    result['b:nontrivial'] = new[1], re.sub(r'\s+', '', new[2])
    return result
