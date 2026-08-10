//! The generated LaTeX must match the markdown it came from.
//!
//! `docs/SHAPE_VERBS.md` is the single source of truth for the three shape
//! verbs. HELICITY's customer manual used to carry a hand-written second copy,
//! and it drifted in UNDER A DAY: it documented TEXTURE's old fixed verdict
//! bands and omitted PRECEDENCE's significance instrument entirely, while the
//! shipped service had already moved. A customer reading it would have ranked
//! instruments on a number the manual never told them was noise.
//!
//! The manual now `\input`s `docs/generated/shape_verbs.tex`, produced by
//! `scripts/shape_verbs_to_tex.py`. This gate is what makes that arrangement
//! hold: edit the markdown without regenerating and `cargo test` fails.
//!
//! It deliberately does NOT shell out to Python. A gate that skips when a
//! toolchain is missing is barely a gate, so the generator stamps the source
//! hash into the file's banner and this test recomputes it from the markdown.
//! No interpreter, no dependency on the developer's PATH.

use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn generated_shape_verbs_tex_matches_the_markdown() {
    let root = repo_root();
    let md_path = root.join("docs").join("SHAPE_VERBS.md");
    let tex_path = root.join("docs").join("generated").join("shape_verbs.tex");

    let md = std::fs::read(&md_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", md_path.display()));
    let tex = std::fs::read_to_string(&tex_path).unwrap_or_else(|e| {
        panic!(
            "cannot read {}: {e}\n\
             Generate it with:  python scripts/shape_verbs_to_tex.py",
            tex_path.display()
        )
    });

    // The generator writes the markdown's hash into the banner. Normalise line
    // endings first: git may check the markdown out with CRLF, and the
    // generator hashes whatever bytes it read at generation time.
    let md_text = String::from_utf8_lossy(&md).replace("\r\n", "\n");
    let expected = sha256_hex(md_text.as_bytes());

    let stamped = tex
        .lines()
        .find_map(|l| l.trim().strip_prefix("%% source-sha256:"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            panic!(
                "{} has no `%% source-sha256:` banner line.\n\
                 It was probably hand-written or hand-edited. Regenerate it:\n  \
                 python scripts/shape_verbs_to_tex.py",
                tex_path.display()
            )
        });

    assert_eq!(
        stamped,
        expected,
        "\n\ndocs/generated/shape_verbs.tex is STALE.\n\
         docs/SHAPE_VERBS.md has changed since it was generated, so the customer \
         manual is now describing behaviour the markdown no longer claims — which \
         is the exact drift this gate exists to prevent.\n\n\
         Fix:  python scripts/shape_verbs_to_tex.py\n\
         Then copy docs/generated/shape_verbs.tex next to the manual .tex.\n\n\
         stamped in the .tex : {stamped}\n\
         current markdown    : {expected}\n"
    );
}

/// The fragment must stay a FRAGMENT. If someone regenerates it with a preamble
/// or a document body, `\input` into the manual breaks at build time — and the
/// manual lives in another repo, so nothing here would otherwise notice.
#[test]
fn generated_fragment_has_no_preamble() {
    let tex = std::fs::read_to_string(
        repo_root().join("docs").join("generated").join("shape_verbs.tex"),
    )
    .expect("read generated fragment");

    for forbidden in [
        "\\documentclass",
        "\\begin{document}",
        "\\end{document}",
        "\\usepackage",
    ] {
        assert!(
            !tex.contains(forbidden),
            "the generated fragment contains `{forbidden}` — it is \\input into a \
             host document and must carry no preamble"
        );
    }

    // It should still be substantive: a converter that silently produced an
    // empty file would pass every check above.
    assert!(
        tex.lines().count() > 200,
        "generated fragment is only {} lines — the converter probably failed to \
         match the markdown's structure",
        tex.lines().count()
    );
    assert!(
        tex.contains("\\section{"),
        "generated fragment has no sections"
    );
}
