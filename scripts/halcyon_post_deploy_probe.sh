#!/usr/bin/env bash
# halcyon_post_deploy_probe.sh
#
# Bash variant of the Halcyon post-deploy verification probe.
# Fires the canonical Halcyon 200-sweep chain on the buckyball at
# fixed seed against the deployed gigi-stream substrate and
# verifies three receipts:
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
#   export GIGI_API_KEY="$(flyctl ssh console -C 'printenv GIGI_API_KEY')"
#   ./scripts/halcyon_post_deploy_probe.sh
#
# Optional override (defaults to production):
#   export GIGI_BASE_URL="https://gigi-stream.fly.dev"

set -u

BASE_URL="${GIGI_BASE_URL:-https://gigi-stream.fly.dev}"
API_KEY="${GIGI_API_KEY:-}"
EXPECTED_SHA="ea7b934ca3fbe9897e9f11851647388972004a2ca025100179a92dd966516591"
EXPECTED_MP="0.5125429110231062"
EPSILON="1e-12"
WALL_TARGET_MS=25
WALL_FAIL_MS=50

if [[ -z "$API_KEY" ]]; then
    echo "FAIL: GIGI_API_KEY is not set." >&2
    echo "  Recover with: flyctl ssh console -C 'printenv GIGI_API_KEY'" >&2
    exit 2
fi

for tool in curl jq python3; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "FAIL: required tool '$tool' not on PATH." >&2
        exit 2
    fi
done

GQL_ENDPOINT="$BASE_URL/v1/gql"

# Single-quoted 'S2' per parser convention from V.0 probe.
STMT_LATTICE="LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';"
STMT_FIELD="GAUGE_FIELD halcyon_canonical_U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY PERSIST;"
STMT_GIBBS="GIBBS_SAMPLE halcyon_canonical_U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;"
STMT_SNAP="SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;"

# Returns: <elapsed_ms>|<http_status>|<body>
# Times curl's wall clock, not the substrate's internal timer.
post_gql() {
    local query="$1"
    local timeout_sec="${2:-120}"
    local body
    body="$(jq -nc --arg q "$query" '{query: $q}')"

    local tmp_body tmp_status t0 t1 elapsed_ms
    tmp_body="$(mktemp)"
    tmp_status="$(mktemp)"
    t0="$(date +%s%N)"
    curl -sS -X POST "$GQL_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Accept: application/json" \
        -H "Authorization: Bearer $API_KEY" \
        -H "X-API-Key: $API_KEY" \
        --max-time "$timeout_sec" \
        --data "$body" \
        -o "$tmp_body" \
        -w '%{http_code}' > "$tmp_status"
    local curl_rc=$?
    t1="$(date +%s%N)"
    elapsed_ms=$(( (t1 - t0) / 1000000 ))

    local status
    status="$(cat "$tmp_status")"
    local resp_body
    resp_body="$(cat "$tmp_body")"
    rm -f "$tmp_body" "$tmp_status"

    if [[ $curl_rc -ne 0 ]]; then
        echo "${elapsed_ms}|000|curl_rc=${curl_rc}"
        return
    fi
    echo "${elapsed_ms}|${status}|${resp_body}"
}

echo "Halcyon post-deploy probe"
echo "  target  : $BASE_URL"
echo "  field   : halcyon_canonical_U"
echo "  beta    : 2.5"
echo "  sweeps  : 200"
echo "  seed    : 20260616"
echo

# ── Step 1: LATTICE declaration. Idempotent on already-declared
#    lattices; non-fatal if the substrate has it cached.
echo "[1/4] LATTICE buckyball ..."
R1="$(post_gql "$STMT_LATTICE")"
STATUS1="$(echo "$R1" | awk -F'|' '{print $2}')"
if [[ "$STATUS1" != "200" ]]; then
    echo "      non-200 ($STATUS1) — continuing (lattice may already be declared)"
fi

# ── Step 2: GAUGE_FIELD declaration. Idempotent under PERSIST.
echo "[2/4] GAUGE_FIELD halcyon_canonical_U ..."
R2="$(post_gql "$STMT_FIELD")"
STATUS2="$(echo "$R2" | awk -F'|' '{print $2}')"
if [[ "$STATUS2" != "200" ]]; then
    echo "      non-200 ($STATUS2) — continuing (field may already be declared)"
fi

# ── Step 3: GIBBS_SAMPLE — THE TIMED STATEMENT. Substrate wall
#    time is the curl round-trip on this call. The response
#    carries the MeanPlaquette chain as a Vector under the
#    `MeanPlaquette` column (src/parser.rs:9188 lowering of
#    ObservableId::MeanPlaquette.label() at src/gauge/gibbs_sample.rs:117).
echo "[3/4] GIBBS_SAMPLE (200 sweeps, seed=20260616) ..."
R3="$(post_gql "$STMT_GIBBS" 600)"
WALL_MS="$(echo "$R3" | awk -F'|' '{print $1}')"
STATUS3="$(echo "$R3" | awk -F'|' '{print $2}')"
BODY3="$(echo "$R3" | cut -d'|' -f3-)"
if [[ "$STATUS3" != "200" ]]; then
    echo "FAIL: GIBBS_SAMPLE returned HTTP $STATUS3" >&2
    echo "$BODY3" >&2
    exit 1
fi

ROW_COUNT="$(echo "$BODY3" | jq '.count // 0')"
if [[ "$ROW_COUNT" -lt 1 ]]; then
    echo "FAIL: GIBBS_SAMPLE response missing rows envelope." >&2
    echo "$BODY3" >&2
    exit 1
fi

CHAIN_LEN="$(echo "$BODY3" | jq '.rows[0].MeanPlaquette | length')"
if [[ "$CHAIN_LEN" -ne 200 ]]; then
    echo "FAIL: MeanPlaquette chain wrong length: got $CHAIN_LEN, want 200." >&2
    exit 1
fi

MP_0="$(echo "$BODY3"   | jq -r '.rows[0].MeanPlaquette[0]')"
MP_199="$(echo "$BODY3" | jq -r '.rows[0].MeanPlaquette[199]')"

# ── Step 4: SNAPSHOT — captures the WAL fingerprint of the
#    thermalized buffer. Response shape (parser.rs:9640-9660):
#   {"rows": [ { "field": "halcyon_canonical_U",
#                "n_edges": 90, "repr_dim": 4,
#                "sha256": "<64-hex>", "wal_offset": <i64> } ],
#    "count": 1}
echo "[4/4] SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST ..."
R4="$(post_gql "$STMT_SNAP")"
STATUS4="$(echo "$R4" | awk -F'|' '{print $2}')"
BODY4="$(echo "$R4" | cut -d'|' -f3-)"
if [[ "$STATUS4" != "200" ]]; then
    echo "FAIL: SNAPSHOT returned HTTP $STATUS4" >&2
    echo "$BODY4" >&2
    exit 1
fi

SNAP_SHA="$(echo "$BODY4"   | jq -r '.rows[0].sha256')"
WAL_OFFSET="$(echo "$BODY4" | jq -r '.rows[0].wal_offset')"
N_EDGES="$(echo "$BODY4"    | jq -r '.rows[0].n_edges')"
REPR_DIM="$(echo "$BODY4"   | jq -r '.rows[0].repr_dim')"

echo
echo "── Receipts ─────────────────────────────────────────────"
echo "  substrate wall      : ${WALL_MS} ms"
echo "  MeanPlaquette[0]    : ${MP_0}"
echo "  MeanPlaquette[199]  : ${MP_199}"
echo "  n_edges             : ${N_EDGES}"
echo "  repr_dim            : ${REPR_DIM}"
echo "  snapshot SHA-256    : ${SNAP_SHA}"
echo "  wal_offset          : ${WAL_OFFSET}"
echo

# ── Assertions ──────────────────────────────────────────────────
FAILURES=()
WARNINGS=()

# A1: substrate wall time. Sprint A target is < 25 ms (7 ms substrate
#     + ~10-30 ms RTT). > 50 ms means Sprint A baseline is not
#     deployed (pre-Sprint-A binary or cold cache).
if [[ "$WALL_MS" -gt "$WALL_FAIL_MS" ]]; then
    FAILURES+=("substrate_wall: ${WALL_MS} ms > ${WALL_FAIL_MS} ms (Sprint A baseline not deployed)")
elif [[ "$WALL_MS" -gt "$WALL_TARGET_MS" ]]; then
    WARNINGS+=("substrate_wall: ${WALL_MS} ms > target ${WALL_TARGET_MS} ms (degraded but within fail bound)")
fi

# A2: MeanPlaquette[199] equals expected within machine epsilon.
#     Use python for float arithmetic; bash has no fp comparison.
MP_DELTA_OK="$(python3 - "$MP_199" "$EXPECTED_MP" "$EPSILON" <<'PY'
import sys
got, want, eps = (float(sys.argv[1]), float(sys.argv[2]), float(sys.argv[3]))
print("OK" if abs(got - want) <= eps else f"FAIL delta={abs(got-want):.3e}")
PY
)"
if [[ "$MP_DELTA_OK" != "OK" ]]; then
    FAILURES+=("MeanPlaquette[199]: got ${MP_199}, want ${EXPECTED_MP} (${MP_DELTA_OK})")
fi

# A3: snapshot SHA equals Halcyon's expected canonical.
SNAP_SHA_LC="$(echo "$SNAP_SHA" | tr '[:upper:]' '[:lower:]')"
if [[ "$SNAP_SHA_LC" != "$EXPECTED_SHA" ]]; then
    FAILURES+=("snapshot_sha256: got ${SNAP_SHA_LC}, want ${EXPECTED_SHA}")
fi

# ── Summary ─────────────────────────────────────────────────────
echo "── Verdict ──────────────────────────────────────────────"
for w in "${WARNINGS[@]:-}"; do
    [[ -n "$w" ]] && echo "  WARN  $w"
done

if [[ ${#FAILURES[@]} -eq 0 ]]; then
    SHA_PREFIX="${SNAP_SHA_LC:0:8}"
    echo "  SHIPPED"
    echo
    echo "  One-line summary (paste into Halcyon reply if Bee chooses):"
    echo "    Post-deploy receipts: substrate_wall=${WALL_MS}ms, MeanPlaquette[199]=${MP_199}, snapshot SHA matches ${SHA_PREFIX}... OK"
    exit 0
else
    echo "  FAIL"
    for f in "${FAILURES[@]}"; do
        echo "    - $f"
    done
    exit 1
fi
