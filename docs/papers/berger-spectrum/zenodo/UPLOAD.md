# Version 2 (new version of the published record)

1. Open the published record (doi:10.5281/zenodo.24386614) and click **New version**. Zenodo keeps both versions
   under the concept DOI 10.5281/zenodo.24386613, which the paper now cites, so no DOI needs reserving.
2. Delete version 1's files. Upload `paper.pdf`, `floor-theorem-v2-latex.zip`,
   `floor-theorem-v2-code-and-data.zip` and `MANIFEST.txt`.
3. **Change the title.** The new-version form keeps version 1's subtitle. Set it to *The Floor Theorem for the
   Yang–Mills Hessian at the Berger Field: A Floor Law Across the Squashed Three-Spheres*.
   Then paste `description.html` through the editor's **Source code** button, and set the version field to **2**.
4. Fix the affiliation. The published version 1 lists **Brown University**; set **Davis Geometric**, as printed
   in the paper.
5. Keywords: add the two new ones from `metadata.json`. Related works are as before.
6. Publish.

---

# Uploading *The Floor Theorem* to Zenodo

The paper says its snapshot "will be archived with a DOI on publication". The clean way to keep that promise is
to reserve the DOI first, write it into the paper, and only then upload.

## 1. Reserve the DOI (Zenodo, about 1 minute)

1. Go to zenodo.org, then **New upload**.
2. Under **Digital Object Identifier**, answer "No" to "Do you already have a DOI?", then click **Get a DOI now!**.
3. Copy the reserved DOI (`10.5281/zenodo.…`). Click **Save draft**, not Publish.

## 2. Put the DOI into the paper and rebuild the package

```
scripts/make_zenodo_floor_theorem.sh ~/Desktop/zenodo-floor-theorem-v1 10.5281/zenodo.NNNNNNNN /path/to/python
```

This rewrites the availability line to cite the DOI, rebuilds `paper.pdf`, and builds the archives and
`MANIFEST.txt`. Then it checks that the LaTeX compiles from its zip and that the proof scripts, rerun from the
code zip in an empty folder, reproduce their committed receipts. After it passes, commit the paper and tag:

```
git add -f docs/papers/berger-spectrum/paper.tex docs/papers/berger-spectrum/paper.pdf
git commit -m "Floor Theorem paper: Zenodo DOI"
git tag floor-theorem-v1 && git push origin HEAD floor-theorem-v1
```

The record's description and metadata point at the tag `floor-theorem-v1`, so it must exist when the record goes
live. Rebuild once more after tagging if you want the manifest to name the tagged commit.

## 3. Fill in the draft

Upload these four files:

- `paper.pdf`
- `floor-theorem-v1-latex.zip`
- `floor-theorem-v1-code-and-data.zip`
- `MANIFEST.txt`

Fill in the form from `metadata.json`:

| Field | Value |
| --- | --- |
| Resource type | Publication, then Preprint |
| Title | The Floor Theorem for the Yang–Mills Hessian at the Berger Field: A Floor Law Across the Squashed Three-Spheres |
| Creator | Davis, Bee Rosa. ORCID 0009-0009-8034-4308. Affiliation: Davis Geometric |
| Description | paste `description.html`, using the editor's **Source code** button so the headings and lists survive |
| Licence | Creative Commons Attribution 4.0 International |
| Keywords | the ten in `metadata.json` |
| Version | 1 |
| Language | English |
| Related works | *References* 10.5281/zenodo.24328085 (Reading the Curve); *References* 10.5281/zenodo.24375338 (Three Ways Across a Turn); *Is supplemented by* https://github.com/nurdymuny/gigi/tree/floor-theorem-v1 (software) |
| Software | Repository URL https://github.com/nurdymuny/gigi/tree/floor-theorem-v1; language Python |
| Notes (optional) | MSC 2020: 58J50, 81T13, 58E15, 53C30, 65G30. No funding; no competing interests. |

**Affiliation.** Davis Geometric, as printed in the paper. The *Reading the Curve* record still lists Brown University; Zenodo lets you edit a published record's metadata (Edit, then Publish) without making a new version.

## 4. Publish

Preview the record, then publish. Afterwards, check that the DOI resolves and that the code zip's `README.md`
renders in Zenodo's file preview.
