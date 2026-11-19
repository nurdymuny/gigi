# Три пути через поворот — русское издание

Russian edition of *Three Ways Across a Turn: Chord, Arc and Tangent*
([doi:10.5281/zenodo.24375338](https://doi.org/10.5281/zenodo.24375338)). The Zenodo record also carries a Russian
translation of the book chapter *Thank You, Mr. Perelman* («Спасибо, господин Перельман»); the chapter is
book material and is deliberately not kept in this repository.

| File | What it is |
| --- | --- |
| `paper_ru.pdf` | the paper in Russian (20 pages), built from `paper_ru.tex` + `part1.tex`…`part5.tex` |
| `figures/` | the six figures with Russian labels: `FIG_LANG=ru python ../make_figures.py` |
| `GLOSSARY.md` | the English → Russian terms every section was translated with |
| `zenodo/` | Zenodo description (Russian, with English summary) and metadata |

How it was made: translated section by section against the glossary, then read in full against
the English by a second reader, who corrected meaning and grammar in place (seven fixes); math, numbers, labels and file paths are checked identical to the
source. The English original remains authoritative.

Build (XeLaTeX; style `../../dgstyle-ru.tex`, Palatino with Cyrillic, Russian hyphenation):

    xelatex paper_ru.tex && xelatex paper_ru.tex

Russian hyphenation needs the `babel-russian` and `hyphen-russian` packages and a XeLaTeX format that
includes Russian (on a user-mode TeX Live: add `russian loadhyph-ru.tex` to a user `language.dat`,
then `fmtutil-user --byfmt xelatex`).
