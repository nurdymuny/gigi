# Ack — wiring received, thread in deploy/drain hold pattern

**To:** GGOG / app team
**From:** GIGI fiber team (Bee + Claude)
**Date:** 2026-06-09 (same-day close on this round-trip)
**Re:** Your `LETTER_TO_GIGI_TEAM_protocol_audit_2026-06-09_wired.md` — 567/567 vitest green, wiring landed
**Working evidence:** this letter at `gigi/theory/ggog/LETTER_TO_GGOG_2026-06-09_ack_wired.md`

## TL;DR

Wiring received. Two design moves worth calling out as cleanly right.
We're at the natural pause: nothing on our side until you ping with
deploy results, verify-only drain numbers, or an implementation-time
question that surfaces during the rollout window.

## Two design choices we want to underline

### (i) AAD `to=` comes from `deps.myAccount`, not `msg.to`

> The `to=` slot comes from `deps.myAccount` (resolved from
> `state.identity`), NOT from `msg.to` on the wire.

This is the right call and it's the kind of subtlety that doesn't show
up in spec text. If the AAD's `to=` were rebuilt from `msg.to`, a
tampering relay that rewrites the recipient field gets a
benign-looking AAD that matches its own lie — the auth tag check
becomes circular against the very threat model it's meant to defend.
Deriving from client-side identity state makes the AAD a hard cross-check
on what the receiver believes it is, not on what the wire claims.

Worth a code comment if it doesn't already have one — the rationale is
load-bearing for a future maintainer who might think "why aren't we
just using `msg.to`?"

### (ii) Three-layer resilience on the challenge dance

The 150ms register deferral + runtime `_wasm.sign_register_challenge`
namespace lookup + accept-both legacy fallback compose into a clean
rollout dance:

- Legacy relays with no challenge frame → 150ms deadline elapses → unsigned register goes through
- Stale wasm artifact (no §6b symbol) → throws at call time, not module-load time → register falls back to unsigned cleanly
- Relay rejection (`bad_challenge` / `challenge_required`) → UI bubble + stop-reconnect → no thundering herd

Each layer protects a different failure mode. The runtime symbol
lookup in particular is the kind of thing operators thank past-self for
when an emergency rollback happens.

## What we're holding

Nothing. The thread is at:

- **deploy** ggog-app with the wired client (your move)
- **watch** `/admin/verify-metrics` for the unsigned-register ratio to drain
- **flip** `GGOG_REQUIRE_CHALLENGE=1` when it settles
- **flip** strict-only projection asserts as the verify-only metric for the new projection types lights up

Days-to-weeks worth of clock time, not minutes of work. Ping us if
anything pops up mid-drain that wants a substrate change — otherwise
the next round-trip is your short "flip went green" note.

## Acknowledgements

> Your `aad_mismatch_fails_decrypt` already proves that side in your
> test gate.

Yes — and your vitest `buildDmAad — encrypt-side and decrypt-side
produce identical bytes` test is the symmetric gate that protects
against silent drift between the two construction sites. Together
those two tests cover the AAD-binding surface end-to-end.

> Once the deploy ships and the verify-only window is clean I'll send
> one more short note confirming the flip went green.

Looking forward to it. That note closes the loop on G2 and your #1 —
the rest of G3/G4/G5 is straight rollout per the prior sequencing.

With care,
— GIGI fiber team

— end —
