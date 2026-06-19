# halcyon_post_deploy_probe.ps1
#
# Post-deploy verification probe for the Halcyon substrate. Fires
# the canonical Halcyon 200-sweep chain on the buckyball at fixed
# seed against the deployed gigi-stream substrate and verifies three
# receipts:
#
#   1. substrate wall time (GIBBS_SAMPLE round-trip) < 25 ms target
#      (warn at > 50 ms — Sprint A face_edges hoist not deployed)
#   2. MeanPlaquette[199] equals 0.5125429110231062 (within epsilon)
#   3. SNAPSHOT SHA-256 equals
#      ea7b934ca3fbe9897e9f11851647388972004a2ca025100179a92dd966516591
#
# The SHA is Halcyon's expected WAL-buffer fingerprint at the chain
# endpoint. Per locked decisions D-V-A (LE encoding) and D-V-C
# (SHA-256 over LE-encoded buffer bytes is the citation handle).
#
# GQL surface used (one statement per POST /v1/gql; the endpoint
# parses a single Statement per body):
#
#   LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';
#   GAUGE_FIELD halcyon_canonical_U ON LATTICE buckyball
#     GROUP SU(2) INIT IDENTITY PERSIST;
#   GIBBS_SAMPLE halcyon_canonical_U BETA 2.5 N_SWEEPS 200
#     MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;
#   SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;
#
# Snapshot statement EBNF (from HALCYON_PART_V_SNAPSHOT_GATES §2,
# locked decision D-V-D requires PERSIST):
#
#   snapshot_stmt
#     : "SNAPSHOT" "GAUGE_FIELD" ident "PERSIST" ";"
#     ;
#
# Usage:
#   $env:GIGI_API_KEY = "<key from: flyctl ssh console -C 'printenv GIGI_API_KEY'>"
#   pwsh ./scripts/halcyon_post_deploy_probe.ps1
#
# Optional override (defaults to production):
#   $env:GIGI_BASE_URL = "https://gigi-stream.fly.dev"

[CmdletBinding()]
param(
    [string]$BaseUrl     = $(if ($env:GIGI_BASE_URL) { $env:GIGI_BASE_URL } else { "https://gigi-stream.fly.dev" }),
    [string]$ApiKey      = $env:GIGI_API_KEY,
    [string]$ExpectedSha = "ea7b934ca3fbe9897e9f11851647388972004a2ca025100179a92dd966516591",
    [double]$ExpectedMP  = 0.5125429110231062,
    [double]$Epsilon     = 1e-12,
    [int]   $WallTargetMs = 25,
    [int]   $WallFailMs   = 50
)

$ErrorActionPreference = "Stop"

if (-not $ApiKey) {
    Write-Host "FAIL: GIGI_API_KEY is not set." -ForegroundColor Red
    Write-Host "  Recover with: flyctl ssh console -C 'printenv GIGI_API_KEY'"
    exit 2
}

$GqlEndpoint = "$BaseUrl/v1/gql"
$Headers = @{
    "Content-Type"  = "application/json"
    "Accept"        = "application/json"
    "Authorization" = "Bearer $ApiKey"
    "X-API-Key"     = $ApiKey
}

function Invoke-Gql {
    param(
        [Parameter(Mandatory=$true)][string]$Query,
        [int]$TimeoutSec = 120
    )
    $body = @{ query = $Query } | ConvertTo-Json -Compress
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $resp = Invoke-RestMethod -Uri $GqlEndpoint -Method Post `
            -Headers $Headers -Body $body -TimeoutSec $TimeoutSec
        $sw.Stop()
        return [pscustomobject]@{
            Response    = $resp
            ElapsedMs   = $sw.Elapsed.TotalMilliseconds
            ErrorString = $null
        }
    } catch {
        $sw.Stop()
        $msg = $_.Exception.Message
        $detail = ""
        try {
            if ($_.ErrorDetails.Message) { $detail = $_.ErrorDetails.Message }
        } catch {}
        return [pscustomobject]@{
            Response    = $null
            ElapsedMs   = $sw.Elapsed.TotalMilliseconds
            ErrorString = "$msg  $detail"
        }
    }
}

# ── GQL bundle (one statement per POST; substrate wall time is the
#    GIBBS_SAMPLE round-trip). Single-quoted 'S2' per the parser
#    convention discovered in V.0 probe.
$Stmt_Lattice = "LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';"
$Stmt_Field   = "GAUGE_FIELD halcyon_canonical_U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY PERSIST;"
$Stmt_Gibbs   = "GIBBS_SAMPLE halcyon_canonical_U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;"
$Stmt_Snap    = "SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;"

Write-Host "Halcyon post-deploy probe"
Write-Host "  target  : $BaseUrl"
Write-Host "  field   : halcyon_canonical_U"
Write-Host "  beta    : 2.5"
Write-Host "  sweeps  : 200"
Write-Host "  seed    : 20260616"
Write-Host ""

# ── Step 1: LATTICE declaration. Idempotent on already-declared
#    lattices; non-fatal if the substrate has it cached.
Write-Host "[1/4] LATTICE buckyball ..."
$r1 = Invoke-Gql -Query $Stmt_Lattice
if ($r1.ErrorString) {
    Write-Host "      $($r1.ErrorString)" -ForegroundColor Yellow
    Write-Host "      (continuing — lattice may already be declared on the substrate)"
}

# ── Step 2: GAUGE_FIELD halcyon_canonical_U declaration. PERSIST
#    means the WAL holds the declaration; subsequent calls re-bind.
Write-Host "[2/4] GAUGE_FIELD halcyon_canonical_U ..."
$r2 = Invoke-Gql -Query $Stmt_Field
if ($r2.ErrorString) {
    Write-Host "      $($r2.ErrorString)" -ForegroundColor Yellow
    Write-Host "      (continuing — field may already be declared on the substrate)"
}

# ── Step 3: GIBBS_SAMPLE — THE TIMED STATEMENT. Substrate wall
#    time is measured here. Response carries the MeanPlaquette
#    chain as Vector under `MeanPlaquette` column per
#    src/parser.rs:9188 + ObservableId::MeanPlaquette.label() at
#    src/gauge/gibbs_sample.rs:117.
Write-Host "[3/4] GIBBS_SAMPLE (200 sweeps, seed=20260616) ..."
$r3 = Invoke-Gql -Query $Stmt_Gibbs -TimeoutSec 600
if ($r3.ErrorString) {
    Write-Host "FAIL: GIBBS_SAMPLE failed:" -ForegroundColor Red
    Write-Host "      $($r3.ErrorString)"
    exit 1
}

$wallMs = [math]::Round($r3.ElapsedMs, 2)
$gibbs  = $r3.Response

# Extract MeanPlaquette[199] from the Rows envelope. Shape:
#   {"rows": [ { "field": "...", "seed": 20260616, "beta": 2.5,
#                "n_sweeps_completed": 200,
#                "MeanPlaquette": [v0, v1, ..., v199] } ],
#    "count": 1}
if (-not $gibbs.rows -or $gibbs.rows.Count -lt 1) {
    Write-Host "FAIL: GIBBS_SAMPLE response missing rows envelope." -ForegroundColor Red
    Write-Host ($gibbs | ConvertTo-Json -Depth 6)
    exit 1
}
$row = $gibbs.rows[0]
$chain = $row.MeanPlaquette
if (-not $chain -or $chain.Count -ne 200) {
    Write-Host "FAIL: MeanPlaquette chain wrong length: got $($chain.Count), want 200." -ForegroundColor Red
    exit 1
}
$mp199 = [double]$chain[199]

# ── Step 4: SNAPSHOT — captures the WAL fingerprint of the
#    thermalized buffer. Response shape (parser.rs:9640-9660):
#   {"rows": [ { "field": "halcyon_canonical_U",
#                "n_edges": 90, "repr_dim": 4,
#                "sha256": "<64-hex>", "wal_offset": <i64> } ],
#    "count": 1}
Write-Host "[4/4] SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST ..."
$r4 = Invoke-Gql -Query $Stmt_Snap
if ($r4.ErrorString) {
    Write-Host "FAIL: SNAPSHOT failed:" -ForegroundColor Red
    Write-Host "      $($r4.ErrorString)"
    exit 1
}
$snap = $r4.Response
if (-not $snap.rows -or $snap.rows.Count -lt 1) {
    Write-Host "FAIL: SNAPSHOT response missing rows envelope." -ForegroundColor Red
    Write-Host ($snap | ConvertTo-Json -Depth 6)
    exit 1
}
$snapRow   = $snap.rows[0]
$snapSha   = [string]$snapRow.sha256
$walOffset = $snapRow.wal_offset
$nEdges    = $snapRow.n_edges
$reprDim   = $snapRow.repr_dim

Write-Host ""
Write-Host "── Receipts ─────────────────────────────────────────────"
Write-Host ("  substrate wall      : {0} ms" -f $wallMs)
Write-Host ("  MeanPlaquette[0]    : {0}" -f ([double]$chain[0]))
Write-Host ("  MeanPlaquette[199]  : {0}" -f $mp199)
Write-Host ("  n_edges             : {0}" -f $nEdges)
Write-Host ("  repr_dim            : {0}" -f $reprDim)
Write-Host ("  snapshot SHA-256    : {0}" -f $snapSha)
Write-Host ("  wal_offset          : {0}" -f $walOffset)
Write-Host ""

# ── Assertions ──────────────────────────────────────────────────
$failures = New-Object System.Collections.Generic.List[string]
$warnings = New-Object System.Collections.Generic.List[string]

# A1: substrate wall time. Sprint A target is < 25 ms (7 ms substrate
#     + ~10-30 ms RTT). > 50 ms means Sprint A baseline is not
#     deployed (pre-Sprint-A binary or cold cache).
if ($wallMs -gt $WallFailMs) {
    $failures.Add("substrate_wall: ${wallMs} ms > ${WallFailMs} ms (Sprint A baseline not deployed)")
} elseif ($wallMs -gt $WallTargetMs) {
    $warnings.Add("substrate_wall: ${wallMs} ms > target ${WallTargetMs} ms (degraded but within fail bound)")
}

# A2: MeanPlaquette[199] equals expected within machine epsilon.
$mpDelta = [math]::Abs($mp199 - $ExpectedMP)
if ($mpDelta -gt $Epsilon) {
    $failures.Add("MeanPlaquette[199]: got $mp199, want $ExpectedMP (delta=$mpDelta > $Epsilon)")
}

# A3: snapshot SHA equals Halcyon's expected canonical.
$snapShaLc = $snapSha.ToLowerInvariant()
if ($snapShaLc -ne $ExpectedSha) {
    $failures.Add("snapshot_sha256: got $snapShaLc, want $ExpectedSha")
}

# ── Summary ─────────────────────────────────────────────────────
Write-Host "── Verdict ──────────────────────────────────────────────"
foreach ($w in $warnings) { Write-Host "  WARN  $w" -ForegroundColor Yellow }

if ($failures.Count -eq 0) {
    $shaPrefix = $snapShaLc.Substring(0, 8)
    $oneLine = ("Post-deploy receipts: substrate_wall={0}ms, " +
                "MeanPlaquette[199]={1}, snapshot SHA matches {2}... OK") -f `
                $wallMs, $mp199, $shaPrefix
    Write-Host "  SHIPPED" -ForegroundColor Green
    Write-Host ""
    Write-Host "  One-line summary (paste into Halcyon reply if Bee chooses):"
    Write-Host "    $oneLine"
    exit 0
} else {
    Write-Host "  FAIL" -ForegroundColor Red
    foreach ($f in $failures) { Write-Host "    - $f" -ForegroundColor Red }
    exit 1
}
