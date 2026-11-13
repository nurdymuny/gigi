# Measured RC checkpoint

This evidence-only checkpoint was requested before investigation of the three
settling disagreements. It is not a release or a hardware-gate closure.

`report.json` identifies the tested binary and every engine source file by
SHA256, as well as the input exports by URL and digest. The tested engine includes
earlier uncommitted prerequisites; this commit does not claim that its own Git
tree alone reconstructs that engine. Unrelated engine work remains untouched.

The helper RED receipt, final tests/mechanism removals, and focused Rust test
log are preserved here. Original third-party measurements are not redistributed.
Full local HTTP receipts and raw downloads remain under
`output/conduct-settle/measured-20261113/`. The corresponding protocol and results
are in `docs/research/`. CS08 remains OPEN.
