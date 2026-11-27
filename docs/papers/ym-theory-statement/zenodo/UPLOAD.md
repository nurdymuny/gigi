# Uploading *The Lowest Note* to Zenodo

This drop **replaces** my earlier posting, which claimed a proof of the Yang–Mills mass gap. It goes up as a
**new version of that record**, not as a new record. The concept DOI stays the same, so every existing link lands on
the replacement. The withdrawn version stays in the record's history, marked as superseded.

## 0. Before building
- Find the published record of the withdrawn proof. You need its **version DOI** (the one shown on the record page)
  and the next **version number**.
- Tag the repository at the revision being posted: `git tag lowest-note-v1 && git push origin lowest-note-v1`.
  The description and metadata link to that tag.

## 1. Build and verify
```
RUST=1 scripts/make_zenodo_lowest_note.sh ~/Desktop/zenodo-lowest-note 10.5281/zenodo.OLDRECORD 2 /path/to/python
```
This builds the PDF and both archives, writes `MANIFEST.txt`, and fills the old DOI and the version into
`metadata.json`. It then unzips everything into empty folders and checks four things:
1. the LaTeX compiles;
2. the threshold reproduces its receipt;
3. the background dressing reproduces `background_deep.json` from the saved spectra;
4. with `RUST=1`, the Rust solver builds and passes the exact round-sphere check.

Upload only after it ends with `built:`.

## 2. On Zenodo
1. Open the published record of the withdrawn proof and click **New version**.
2. **Delete the old files.** Upload the four files in `zenodo-upload/`: `the-lowest-note.pdf`,
   `the-lowest-note-latex.zip`, `the-lowest-note-code-and-data.zip`, `MANIFEST.txt`.
3. **Title:** *The Lowest Note: A Geometric Theory of the Yang–Mills Mass Gap*. The new-version form keeps the old
   title, so change it.
4. **Description:** paste `description.html` through the editor's **Source code** button. Its first paragraph is the
   withdrawal; keep it first.
5. **Version:** the number you passed to the builder.
6. **Creator:** Davis, Bee Rosa; affiliation **Davis Geometric**; ORCID 0009-0009-8034-4308.
7. **Keywords, notes and related works:** take them from `metadata.json`. *Is new version of* the old version DOI.
   *References* By Enough, the Floor theorem and Three Ways. *Is supplemented by* the `lowest-note-v1` tag.
8. **License:** CC BY 4.0. Publish.

## 3. After publishing
- Check the four files against `MANIFEST.txt` (Zenodo shows MD5; recompute locally if needed).
- Record the new version DOI at the top of this file.
