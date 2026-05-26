"""SUDOKU primitive on a literal 32x32 sudoku puzzle.

The primitive's namesake. This demonstrates SUDOKU at *literal*
sudoku scale: a 32x32 grid (1024 cells) with 32 digits (1..32),
4-row x 8-col rectangular regions, and SUDOKU acting as the
per-cell oracle inside a constraint-propagation loop.

Bundle: 32 records, one per digit (just `{digit: i}` for i=1..32).
Query per cell: up to ~90 `!=` constraints from same-row,
same-col, and same-region cells already filled in.

Output: solve time + final state.

Run against a local gigi-stream on port 3143.
"""
import http.client
import json
import sys
import time

HOST, PORT = "localhost", 3143
N = 32                                   # grid size
REGION_ROWS, REGION_COLS = 4, 8           # 4 rows x 8 cols per region
ASSERT_N_REGION = REGION_ROWS * REGION_COLS
assert ASSERT_N_REGION == N, "region must cover N cells"

# ─────────────────────────────────────────────────────────────────
# HTTP plumbing -- uses keep-alive connection for 4000x speedup
# vs urllib.urlopen (per-call cost: 0.5ms vs 2000ms on Windows).
# This matters at scale: 1000 sudoku cells * 2s = unusable;
# 1000 cells * 0.5ms = instant.
# ─────────────────────────────────────────────────────────────────

CONN = http.client.HTTPConnection(HOST, PORT, timeout=30)
HEADERS = {"Content-Type": "application/json", "Connection": "keep-alive"}

def post(path, body):
    payload = json.dumps(body)
    try:
        CONN.request("POST", path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        return resp.status, json.loads(data) if data else {}
    except (http.client.BadStatusLine, ConnectionResetError):
        # Reconnect on dropped keep-alive.
        CONN.close()
        CONN.connect()
        CONN.request("POST", path, payload, HEADERS)
        resp = CONN.getresponse()
        data = resp.read()
        return resp.status, json.loads(data) if data else {}


def setup_digit_bundle():
    """Create the digit-bundle: 32 records, one per digit."""
    schema = {
        "name": "sudoku_digits",
        "schema": {"fields": {"digit": "numeric"}, "keys": ["digit"]},
    }
    post("/v1/bundles", schema)            # idempotent
    records = [{"digit": i} for i in range(1, N + 1)]
    post("/v1/bundles/sudoku_digits/insert", {"records": records})


# ─────────────────────────────────────────────────────────────────
# Puzzle generation
# ─────────────────────────────────────────────────────────────────

def solved_grid():
    """Generate a valid solved 32x32 grid using the canonical
    band-shift construction. Verified by hand: every row, col, and
    4x8 region contains exactly {1..32}."""
    grid = [[0] * N for _ in range(N)]
    for r in range(N):
        for c in range(N):
            grid[r][c] = ((r % REGION_ROWS) * REGION_COLS + (r // REGION_ROWS) + c) % N + 1
    return grid


def verify(grid):
    """Sanity-check: every row, col, region is a permutation of 1..N."""
    target = set(range(1, N + 1))
    for r in range(N):
        if set(grid[r]) != target:
            return f"row {r} broken: {sorted(grid[r])}"
    for c in range(N):
        col = [grid[r][c] for r in range(N)]
        if set(col) != target:
            return f"col {c} broken: {sorted(col)}"
    for rb in range(0, N, REGION_ROWS):
        for cb in range(0, N, REGION_COLS):
            region = [grid[rb + i][cb + j]
                      for i in range(REGION_ROWS)
                      for j in range(REGION_COLS)]
            if set(region) != target:
                return f"region ({rb},{cb}) broken"
    return "ok"


def knock_out(solved, percent_empty, seed=42):
    """Create a puzzle by replacing `percent_empty`% of cells with 0."""
    import random
    rng = random.Random(seed)
    puzzle = [row[:] for row in solved]
    cells = [(r, c) for r in range(N) for c in range(N)]
    rng.shuffle(cells)
    n_empty = int(N * N * percent_empty / 100)
    for r, c in cells[:n_empty]:
        puzzle[r][c] = 0
    return puzzle


# ─────────────────────────────────────────────────────────────────
# Constraint building
# ─────────────────────────────────────────────────────────────────

def region_of(r, c):
    """Return (region_row_start, region_col_start) for cell (r, c)."""
    return (r // REGION_ROWS) * REGION_ROWS, (c // REGION_COLS) * REGION_COLS


def constraints_for_cell(puzzle, r, c):
    """Build the `!= value` constraint list for cell (r, c).

    Walks row, col, and region of the cell; for every non-empty
    cell, emits a constraint `digit != value`. Returns a list of
    wire-format constraint dicts.
    """
    forbidden = set()
    # Row.
    for cc in range(N):
        if puzzle[r][cc] != 0 and cc != c:
            forbidden.add(puzzle[r][cc])
    # Col.
    for rr in range(N):
        if puzzle[rr][c] != 0 and rr != r:
            forbidden.add(puzzle[rr][c])
    # Region.
    rb, cb = region_of(r, c)
    for rr in range(rb, rb + REGION_ROWS):
        for cc in range(cb, cb + REGION_COLS):
            if puzzle[rr][cc] != 0 and (rr, cc) != (r, c):
                forbidden.add(puzzle[rr][cc])
    return [
        {"type": "field", "field": "digit", "op": "ne",
         "value": v, "hard": True}
        for v in sorted(forbidden)
    ]


# ─────────────────────────────────────────────────────────────────
# Solver loop — SUDOKU as the per-cell oracle
# ─────────────────────────────────────────────────────────────────

def solve_with_sudoku_primitive(puzzle, max_passes=5, verbose=True):
    """Iterate: for every empty cell, query SUDOKU. If SUDOKU
    returns exactly ONE solution, fill it in. Repeat until no
    progress is made.

    Returns (solved_grid, stats).
    """
    grid = [row[:] for row in puzzle]
    stats = {
        "n_initial_empty": sum(1 for r in range(N) for c in range(N) if grid[r][c] == 0),
        "n_filled_by_propagation": 0,
        "n_unresolved": 0,
        "sudoku_calls": 0,
        "total_ms": 0.0,
        "max_constraints_per_query": 0,
        "passes_used": 0,
    }
    started = time.time()
    for pass_idx in range(max_passes):
        stats["passes_used"] = pass_idx + 1
        filled_this_pass = 0
        for r in range(N):
            for c in range(N):
                if grid[r][c] != 0:
                    continue
                constraints = constraints_for_cell(grid, r, c)
                stats["max_constraints_per_query"] = max(
                    stats["max_constraints_per_query"], len(constraints))
                stats["sudoku_calls"] += 1
                status, body = post(
                    "/v1/bundles/sudoku_digits/brain/sudoku",
                    {"constraints": constraints,
                     "max_options": 2,
                     "max_near_misses": 0},
                )
                if status != 200:
                    print(f"  ERROR at ({r},{c}): HTTP {status}: {body.get('error')}")
                    continue
                sols = body.get("solutions") or []
                if len(sols) == 1:
                    digit = sols[0]["record"]["digit"]
                    grid[r][c] = digit
                    stats["n_filled_by_propagation"] += 1
                    filled_this_pass += 1
        if verbose:
            unresolved = sum(1 for r in range(N) for c in range(N) if grid[r][c] == 0)
            print(f"  pass {pass_idx+1}: filled {filled_this_pass}, "
                  f"{unresolved} unresolved")
        if filled_this_pass == 0:
            break
    stats["total_ms"] = (time.time() - started) * 1000.0
    stats["n_unresolved"] = sum(1 for r in range(N) for c in range(N) if grid[r][c] == 0)
    return grid, stats


# ─────────────────────────────────────────────────────────────────
# Pretty printing
# ─────────────────────────────────────────────────────────────────

def print_grid(grid, title=""):
    if title:
        print(f"\n{title}")
    print("+" + "-" * (N * 3 + (N // REGION_COLS - 1) * 1) + "+")
    for r in range(N):
        row_str = "|"
        for c in range(N):
            v = grid[r][c]
            cell = f"{v:2d} " if v != 0 else " . "
            row_str += cell
            if (c + 1) % REGION_COLS == 0 and (c + 1) < N:
                row_str += "|"
        row_str += "|"
        print(row_str)
        if (r + 1) % REGION_ROWS == 0 and (r + 1) < N:
            print("+" + "-" * (N * 3 + (N // REGION_COLS - 1) * 1) + "+")
    print("+" + "-" * (N * 3 + (N // REGION_COLS - 1) * 1) + "+")


# ─────────────────────────────────────────────────────────────────
# MAIN
# ─────────────────────────────────────────────────────────────────

print("=" * 78)
print("SUDOKU primitive on a literal 32x32 grid (1024 cells, digits 1..32)")
print("Regions: 4 rows x 8 cols (32 cells each, 32 regions total)")
print("=" * 78)

# Build the digit bundle once.
print("\nSetting up digit bundle (32 records)...")
setup_digit_bundle()

# Generate solved grid + verify.
solved = solved_grid()
sanity = verify(solved)
print(f"Sanity-check solved grid: {sanity}")
assert sanity == "ok"

# Knock out cells to make a puzzle. 30% empty is "moderately easy"
# for naive propagation; SUDOKU as oracle should still resolve it.
PERCENT_EMPTY = 30
puzzle = knock_out(solved, PERCENT_EMPTY, seed=42)
n_empty = sum(1 for r in range(N) for c in range(N) if puzzle[r][c] == 0)
print(f"\nPuzzle: {N}x{N} = {N*N} cells, "
      f"{n_empty} empty ({100*n_empty/(N*N):.0f}%).")

# Show a corner of the puzzle so the reader sees what we're solving.
print("\nTop-left 12x12 corner of the puzzle (0 means empty):")
for r in range(12):
    print("  " + " ".join(f"{puzzle[r][c]:2d}" if puzzle[r][c] else " ." for c in range(12)))

# Solve.
print("\nSolving via SUDOKU primitive as per-cell oracle...")
result, stats = solve_with_sudoku_primitive(puzzle, max_passes=5)

print("\n" + "=" * 78)
print("SUMMARY")
print("=" * 78)
print(f"  Empty cells (start):           {stats['n_initial_empty']}")
print(f"  Filled by constraint prop:     {stats['n_filled_by_propagation']}")
print(f"  Unresolved at exit:            {stats['n_unresolved']}")
print(f"  Passes used:                   {stats['passes_used']}")
print(f"  Total SUDOKU calls:            {stats['sudoku_calls']}")
print(f"  Max constraints per query:     {stats['max_constraints_per_query']}")
print(f"  Total wall time:               {stats['total_ms']:.0f} ms")
print(f"  Mean ms/call:                  {stats['total_ms']/max(1,stats['sudoku_calls']):.1f}")

# Verify final state.
final_sanity = "incomplete" if stats["n_unresolved"] > 0 else verify(result)
print(f"  Final-state sanity:            {final_sanity}")

# Match against ground truth.
match = sum(1 for r in range(N) for c in range(N)
            if result[r][c] == solved[r][c])
print(f"  Correct cells (vs ground):     {match} / {N*N} "
      f"({100*match/(N*N):.1f}%)")
print()
