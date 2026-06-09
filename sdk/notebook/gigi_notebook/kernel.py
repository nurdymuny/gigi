"""GIGI Jupyter kernel — GQL by default, magic prefixes for HTTP verbs.

The kernel speaks the standard Jupyter kernel protocol via ipykernel.
Cell text is interpreted as:

- A magic line starting with ``%%commutator`` (etc.) → routes to the
  matching handler, which is typically a thin POST + custom renderer.
- ``%env KEY=value`` → updates the kernel's view of an environment
  variable for this session.
- Otherwise → treated as a GQL statement, sent to the GIGI engine via
  ``POST /v1/gql``, response rendered as JSON.

State (config, last result, last commutator) persists between cells as
ordinary instance attributes on the kernel.

This is intentionally a minimum viable kernel. The renderer set is
small. The plan is to grow magics (``%%transport``, ``%%marcella``,
etc.) and renderers (commutator heatmap, bundle SVG, transport vector
overlay on the belief simplex) one at a time.
"""

from __future__ import annotations

import json
import os
import textwrap
from typing import Any, Dict, Optional

import requests
from ipykernel.kernelbase import Kernel


# ── Magic registry ────────────────────────────────────────────────────────
#
# Each entry maps a ``%%magic`` token to the kernel method that handles it.
# Handlers receive the raw cell body (everything after the magic line) and
# return a dict suitable for ``display_data`` (with ``"text/plain"`` and,
# where useful, additional MIME types).


_MAGIC_PREFIX = "%%"


# ── Kernel ────────────────────────────────────────────────────────────────


class GigiKernel(Kernel):
    """A Jupyter kernel that talks to a running gigi-stream server."""

    implementation = "gigi"
    implementation_version = "0.1.0"
    language = "gql"
    language_version = "0.1"
    language_info = {
        "name": "gql",
        "mimetype": "text/x-gql",
        "file_extension": ".gql",
        "pygments_lexer": "sql",     # close enough for highlighting
    }
    banner = (
        "GIGI Jupyter kernel v0.1.0 — GQL by default.\n"
        "Cell magics: %%commutator. Set GIGI_URL and GIGI_API_KEY in env."
    )

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        # Per-session config snapshot. The user can override these by
        # putting ``%env GIGI_URL=...`` at the top of a cell.
        self._gigi_url = os.environ.get("GIGI_URL", "http://localhost:3142")
        self._api_key = os.environ.get("GIGI_API_KEY", "")
        # State kept across cells.
        self._last_result: Optional[Dict[str, Any]] = None
        # HTTP session for connection pooling. Reused across cells.
        self._http = requests.Session()

    # ── do_execute: the kernel protocol entry point ──────────────────

    def do_execute(
        self,
        code: str,
        silent: bool,
        store_history: bool = True,
        user_expressions: Optional[Dict[str, Any]] = None,
        allow_stdin: bool = False,
        *,
        cell_id: Optional[str] = None,
    ):
        """Called by Jupyter for every cell. Dispatch by leading token."""
        try:
            display = self._dispatch(code)
        except Exception as exc:
            self._send_error(str(exc))
            return {
                "status": "error",
                "execution_count": self.execution_count,
                "ename": type(exc).__name__,
                "evalue": str(exc),
                "traceback": [str(exc)],
            }

        if not silent and display is not None:
            self.send_response(
                self.iopub_socket,
                "display_data",
                {"data": display, "metadata": {}},
            )

        return {
            "status": "ok",
            "execution_count": self.execution_count,
            "payload": [],
            "user_expressions": {},
        }

    # ── Dispatcher: per-cell shape decisions ─────────────────────────

    def _dispatch(self, code: str) -> Optional[Dict[str, Any]]:
        stripped = code.lstrip()
        if not stripped:
            return None

        # %env KEY=value — update session config.
        if stripped.startswith("%env "):
            self._apply_env_line(stripped[len("%env "):].splitlines()[0])
            return {"text/plain": f"# env updated. GIGI_URL={self._gigi_url}"}

        # Magic prefix dispatch.
        if stripped.startswith(_MAGIC_PREFIX):
            first_line, _, body = stripped.partition("\n")
            magic_token = first_line[len(_MAGIC_PREFIX):].split(maxsplit=1)
            magic_name = magic_token[0] if magic_token else ""
            handler = self._magic_handlers().get(magic_name)
            if handler is None:
                raise ValueError(
                    f"unknown magic {first_line!r}; known: "
                    + ", ".join(self._magic_handlers().keys())
                )
            return handler(body)

        # Default: treat as GQL.
        return self._handle_gql(stripped)

    # ── Magic handlers ───────────────────────────────────────────────

    def _magic_handlers(self) -> Dict[str, Any]:
        return {
            "commutator": self._handle_commutator,
            "gql": self._handle_gql,                  # explicit form
            "config": self._handle_config,            # show current config
        }

    def _handle_config(self, _body: str) -> Dict[str, Any]:
        """``%%config`` — show kernel session config."""
        lines = [
            f"GIGI_URL    = {self._gigi_url}",
            f"GIGI_API_KEY= {'<set>' if self._api_key else '<unset>'}",
            f"last_result = {'<present>' if self._last_result else '<none>'}",
        ]
        return {"text/plain": "\n".join(lines)}

    def _handle_gql(self, body: str) -> Dict[str, Any]:
        """Send GQL to /v1/gql and render the response."""
        body = body.strip()
        if not body:
            return {"text/plain": "# empty GQL cell"}
        result = self._post("/v1/gql", {"query": body})
        self._last_result = result
        return self._render_json(result)

    def _handle_commutator(self, body: str) -> Dict[str, Any]:
        """``%%commutator`` — JSON body, POST to /v1/causal_states/commutator."""
        body = body.strip()
        if not body:
            raise ValueError(
                "%%commutator: cell body must be a JSON object with keys "
                "{a, b, base_belief}; see "
                "https://gigi-stream.fly.dev for the schema."
            )
        try:
            payload = json.loads(body)
        except json.JSONDecodeError as e:
            raise ValueError(f"%%commutator: invalid JSON: {e}") from e
        result = self._post("/v1/causal_states/commutator", payload)
        self._last_result = result
        return self._render_commutator(result)

    # ── HTTP helper ──────────────────────────────────────────────────

    def _post(self, path: str, body: Dict[str, Any]) -> Dict[str, Any]:
        url = self._gigi_url.rstrip("/") + path
        headers = {"Content-Type": "application/json"}
        if self._api_key:
            headers["X-API-Key"] = self._api_key
        resp = self._http.post(url, json=body, headers=headers, timeout=30)
        if resp.status_code >= 400:
            try:
                err_body = resp.json()
            except ValueError:
                err_body = {"error": resp.text}
            raise RuntimeError(
                f"HTTP {resp.status_code} on {path}: "
                f"{err_body.get('error', err_body)}"
            )
        try:
            return resp.json()
        except ValueError:
            raise RuntimeError(f"non-JSON response from {path}: {resp.text}")

    # ── Renderers ────────────────────────────────────────────────────

    @staticmethod
    def _render_json(value: Any) -> Dict[str, str]:
        """Generic renderer — JSON pretty-print + plain Python repr."""
        pretty = json.dumps(value, indent=2, default=str)
        return {
            "text/plain": pretty,
            "application/json": pretty,
        }

    @staticmethod
    def _render_commutator(result: Dict[str, Any]) -> Dict[str, str]:
        """Pretty box-drawing for a Commutator response.

        Layout::

            ┌──────────┬───────────────┐
            │  regime  │     smooth    │
            ├──────────┼───────────────┤
            │   TV     │ 0.10619       │
            │  Hel     │ 0.07520       │
            │   KL     │ 0.03266 bits  │
            └──────────┴───────────────┘
            forward : [0.4469, 0.5531]
            backward: [0.5531, 0.4469]
        """
        regime = result.get("regime", "?")
        tv = result.get("tv", float("nan"))
        hel = result.get("hellinger", float("nan"))
        kl_obj = result.get("kl", {})
        if isinstance(kl_obj, dict):
            if kl_obj.get("kind") == "finite":
                kl_repr = f"{kl_obj.get('value', float('nan')):.6f} bits"
            elif kl_obj.get("kind") == "divergent":
                kl_repr = "divergent (sofic)"
            else:
                kl_repr = str(kl_obj)
        else:
            kl_repr = str(kl_obj)

        forward = result.get("forward", [])
        backward = result.get("backward", [])

        def fmt_belief(b):
            if not b:
                return "[]"
            return "[" + ", ".join(f"{v:.4f}" for v in b) + "]"

        # Build the box. Column widths sized to fit a typical line.
        left_w = 10
        right_w = max(15, len(kl_repr) + 2, len(regime) + 4)
        top = f"┌{'─'*left_w}┬{'─'*right_w}┐"
        sep = f"├{'─'*left_w}┼{'─'*right_w}┤"
        bot = f"└{'─'*left_w}┴{'─'*right_w}┘"

        def row(left: str, right: str) -> str:
            return f"│{left.center(left_w)}│{right.center(right_w)}│"

        lines = [
            top,
            row("regime", regime),
            sep,
            row("TV", f"{tv:.6f}"),
            row("Hel", f"{hel:.6f}"),
            row("KL", kl_repr),
            bot,
            f"forward : {fmt_belief(forward)}",
            f"backward: {fmt_belief(backward)}",
        ]
        plain = "\n".join(lines)
        return {
            "text/plain": plain,
            "application/json": json.dumps(result, indent=2),
        }

    # ── Misc ─────────────────────────────────────────────────────────

    def _apply_env_line(self, line: str) -> None:
        """Parse ``KEY=value`` and apply to this session."""
        if "=" not in line:
            raise ValueError(
                f"%env: expected KEY=value, got {line!r}"
            )
        key, _, value = line.partition("=")
        key = key.strip()
        value = value.strip()
        if key == "GIGI_URL":
            self._gigi_url = value
        elif key == "GIGI_API_KEY":
            self._api_key = value
        else:
            # Forward to actual env so subprocesses / other libs see it.
            os.environ[key] = value

    def _send_error(self, message: str) -> None:
        """Send a `stream` message with the error text to the client."""
        self.send_response(
            self.iopub_socket,
            "stream",
            {"name": "stderr", "text": textwrap.dedent(message) + "\n"},
        )


if __name__ == "__main__":
    from ipykernel.kernelapp import IPKernelApp

    IPKernelApp.launch_instance(kernel_class=GigiKernel)
