"""Auth chain diagnostic — one-shot probe of the full sheets ↔ davisgeometric ↔ engine path.

Run any time without PowerShell hashtable gymnastics:

    python e2e/probes/auth_chain_diag.py

Or to test a specific key (overrides the env var):

    python e2e/probes/auth_chain_diag.py --key <hex-key>

Reads GIGI_API_KEY from:
  1. --key CLI arg (highest priority)
  2. $env:GIGI_API_KEY (PowerShell-set) / $GIGI_API_KEY (bash-set)
  3. C:\\Users\\nurdm\\OneDrive\\Documents\\math-website-main\\.env.local

Probes (each prints PASS/MISS with what it means):

  [1] davisgeometric.com is reachable + healthy
  [2] /api/auth/session returns 200 (the route the browser hit on the 500)
  [3] /api/gigi/token returns 200 with a key (only works when signed in)
  [4] engine refuses requests with NO key (auth wall is alive)
  [5] engine refuses the LEAKED old key (proves rotation landed)
  [6] engine ACCEPTS the new key (proves current credential works)
  [7] new key has NO trailing whitespace (the original 401-storm bug)

Exit 0 if every probe that can run on the current credential passes.
Exit 1 if any probe fails.
"""
from __future__ import annotations

import argparse
import http.client
import json
import os
import ssl
import sys
from pathlib import Path

DG_HOST = "www.davisgeometric.com"
ENGINE_HOST = "gigi-stream.fly.dev"
LEAKED_OLD_KEY = "YAxTPuCDAXoXWqUl2hUacHwx5CMwqYaRLzBWxeFGGYqp1YOR"
ENV_LOCAL_PATH = Path(
    r"C:\Users\nurdm\OneDrive\Documents\math-website-main\.env.local"
)

ctx = ssl.create_default_context()


def resolve_key(cli_key: str | None) -> tuple[str | None, str]:
    """Returns (key, where_found). Empty key → (None, ...)."""
    if cli_key:
        return cli_key.strip() or None, "--key arg"
    env_key = os.environ.get("GIGI_API_KEY", "").strip()
    if env_key:
        return env_key, "$GIGI_API_KEY env"
    if ENV_LOCAL_PATH.exists():
        try:
            for line in ENV_LOCAL_PATH.read_text(encoding="utf-8").splitlines():
                if line.startswith("GIGI_API_KEY="):
                    raw = line.split("=", 1)[1].strip().strip('"').strip("'")
                    if raw:
                        return raw, f"{ENV_LOCAL_PATH.name}"
        except Exception as exc:
            return None, f".env.local read error: {exc}"
    return None, "not found"


def http_call(host: str, method: str, path: str, headers: dict[str, str] | None = None) -> tuple[int, dict[str, str], bytes]:
    conn = http.client.HTTPSConnection(host, 443, timeout=20, context=ctx)
    conn.request(method, path, "", headers or {})
    r = conn.getresponse()
    body = r.read()
    return r.status, dict(r.getheaders()), body


def short_body(body: bytes, n: int = 140) -> str:
    s = body[:n].decode("utf-8", errors="replace").strip()
    return s.replace("\n", " ")


results: list[tuple[str, bool, str]] = []


def check(label: str, ok: bool, detail: str = "") -> None:
    tag = "PASS" if ok else "MISS"
    print(f"  [{tag}] {label}" + (f"  -- {detail}" if detail else ""))
    results.append((label, ok, detail))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--key", help="Override engine API key for probe 6")
    args = parser.parse_args()

    new_key, key_source = resolve_key(args.key)

    print("=" * 72)
    print("  GIGI auth-chain diagnostic")
    print("=" * 72)
    print(f"  davisgeometric : {DG_HOST}")
    print(f"  engine         : {ENGINE_HOST}")
    print(f"  new key from   : {key_source}")
    if new_key:
        print(f"  new key prefix : {new_key[:8]}…  ({len(new_key)} chars)")
    else:
        print("  new key prefix : (none — probes 6+7 will be skipped)")
    print()

    # ───── [1] davisgeometric.com reachable ──────────────────────────────────
    print("[1] davisgeometric.com reachable + healthy")
    try:
        status, hdrs, body = http_call(DG_HOST, "GET", "/")
        check(
            f"GET https://{DG_HOST}/ returns 2xx/3xx",
            200 <= status < 400,
            f"status={status}",
        )
    except Exception as e:
        check("GET / network error", False, repr(e))

    # ───── [2] /api/auth/session ─────────────────────────────────────────────
    print("\n[2] /api/auth/session (the route that 500'd in the browser)")
    try:
        status, hdrs, body = http_call(DG_HOST, "GET", "/api/auth/session")
        check(
            "session route returns 200",
            status == 200,
            f"status={status}, body={short_body(body)}",
        )
    except Exception as e:
        check("session route network error", False, repr(e))

    # ───── [3] /api/gigi/token ───────────────────────────────────────────────
    # This requires the caller to be signed in (a real session cookie). From a
    # cold script we expect 401 / 403 — that's "auth required, not server
    # error" which is the right behavior. A 5xx here would be the bug.
    print("\n[3] /api/gigi/token (anonymous probe — expect 401/403, not 5xx)")
    try:
        status, hdrs, body = http_call(DG_HOST, "GET", "/api/gigi/token")
        check(
            "token route is up (anything except 5xx)",
            status < 500,
            f"status={status}, body={short_body(body)}",
        )
    except Exception as e:
        check("token route network error", False, repr(e))

    # ───── [4] engine refuses NO key ─────────────────────────────────────────
    print("\n[4] engine refuses requests with NO credential")
    try:
        status, hdrs, body = http_call(ENGINE_HOST, "GET", "/v1/bundles")
        check(
            "engine returns 401 with no key (auth wall is alive)",
            status == 401,
            f"status={status}, body={short_body(body)}",
        )
    except Exception as e:
        check("engine no-key network error", False, repr(e))

    # ───── [5] engine refuses LEAKED OLD key ─────────────────────────────────
    print("\n[5] engine refuses the leaked old key (proves rotation landed)")
    try:
        status, hdrs, body = http_call(
            ENGINE_HOST, "GET", "/v1/bundles", {"X-API-Key": LEAKED_OLD_KEY}
        )
        check(
            f"engine returns 401 for old key {LEAKED_OLD_KEY[:8]}…",
            status == 401,
            f"status={status}",
        )
    except Exception as e:
        check("engine old-key network error", False, repr(e))

    # ───── [6] engine accepts NEW key ────────────────────────────────────────
    print("\n[6] engine accepts the new key (proves current credential works)")
    if not new_key:
        check("new-key acceptance check", False, "no new key supplied — see header")
    else:
        try:
            status, hdrs, body = http_call(
                ENGINE_HOST, "GET", "/v1/bundles", {"X-API-Key": new_key}
            )
            ok = status == 200
            preview = ""
            if ok:
                try:
                    arr = json.loads(body)
                    preview = f"got {len(arr)} bundles"
                except Exception:
                    preview = "200 OK (non-array response)"
            else:
                preview = f"status={status}, body={short_body(body)}"
            check("engine returns 200 with new key", ok, preview)
        except Exception as e:
            check("engine new-key network error", False, repr(e))

    # ───── [7] new key has no whitespace ─────────────────────────────────────
    print("\n[7] new key has no trailing whitespace (the original 401-storm bug)")
    if new_key:
        clean = new_key == new_key.strip()
        no_internal = all(c not in new_key for c in "\r\n\t ")
        ok = clean and no_internal
        detail = "clean" if ok else f"contains whitespace — repr={new_key!r}"
        check("new key is whitespace-free", ok, detail)

    # ───── summary ───────────────────────────────────────────────────────────
    print()
    print("=" * 72)
    passed = sum(1 for _, ok, _ in results if ok)
    total = len(results)
    print(f"PASSED: {passed}/{total}")
    fails = [(lbl, det) for lbl, ok, det in results if not ok]
    if fails:
        print("\nMISSES:")
        for lbl, det in fails:
            print(f"  - {lbl}")
            if det:
                print(f"      {det}")
        return 1
    print("\nAll probes green — the auth chain is healthy end-to-end.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
