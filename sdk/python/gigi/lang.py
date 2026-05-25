"""
gigi.lang — GIGI Lang client (prompt → GQL → fiber response).

GIGI Lang is the named, documented surface that ties together:

    natural-language prompt
        → translated to GQL
        → executed against GIGI's fiber-bundle store
        → fiber-shaped response (DHOOM by default, JSON optional)
        → returned to caller

The user-facing contract: write a prompt, get back a fiber-shaped response.
The translation layer is the only place where ambiguity is allowed; everything
downstream is lossless.

This module is the **contract skeleton.** All methods raise NotImplementedError.
The signatures, type hints, docstrings, and error envelope are the spec made
executable — implementations land in a subsequent revision against the same
interface, so translation-layer build can proceed against a fixed contract.

Spec reference:
    ~/Documents/gigi/GIGI_LANG_SPEC.md (v0.1.1)

Schema endpoint reference:
    ~/Documents/gigi/GIGI_SCHEMA_INTROSPECTION_SPEC.md

Status:
    v0.0 — skeleton. No method has a working implementation.
    v0.1 — first working ask() / translate() / execute() / query(); requires
           schema endpoint live and translator wired up.
    v0.2 — full surface inventory exposed (Rust-only primitives surfaced via GQL).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional

from .client import GigiClient, GigiError


# ---------------------------------------------------------------------------
# Type aliases (for signature clarity)
# ---------------------------------------------------------------------------

GQLQuery = str
"""A GraphQL query string, valid against GIGI's schema."""

Prompt = str
"""A natural-language prompt to be translated to GQL."""


# ---------------------------------------------------------------------------
# Response types
# ---------------------------------------------------------------------------

@dataclass
class FiberResponse:
    """
    A fiber-shaped response from GIGI.

    Carries the data payload along with the fiber metadata (schema/structure)
    that shaped it. Consumers (LLMs, downstream tools, humans) can interpret
    the response without external schema lookup — the fiber describes itself.

    Attributes:
        data:    The decoded response payload. Shape depends on the query.
                 For DHOOM-format responses, this is the decoded Python value
                 (list of dicts, dict, or scalar). For JSON responses, the
                 same shape via json.loads.
        fiber:   The fiber metadata describing the response structure
                 (field names, types, modifiers). Empty dict if no metadata.
        format:  Serialization format used ("dhoom" or "json").
        raw:     The raw response body as bytes (for callers that want to
                 re-decode, cache, or forward verbatim). Optional; may be None.
    """
    data: Any
    fiber: Dict[str, Any] = field(default_factory=dict)
    format: str = "dhoom"
    raw: Optional[bytes] = None


@dataclass
class TranslationResult:
    """
    The output of translating a prompt to GQL.

    Distinguishes confident translations from ambiguous ones. Ambiguous
    translations include candidate alternatives so the caller can pick or
    refine the prompt.

    Attributes:
        gql:        The primary GQL query the translator produced.
        alternates: Other plausible GQL queries the translator considered.
                    Empty list for confident translations.
        confidence: Translator's self-reported confidence in [0.0, 1.0].
                    Threshold for "ambiguous" is implementation-defined;
                    the contract here is just to surface the score.
        notes:      Optional free-text notes from the translator
                    (e.g., warnings about assumed defaults).
    """
    gql: GQLQuery
    alternates: List[GQLQuery] = field(default_factory=list)
    confidence: float = 1.0
    notes: Optional[str] = None


# ---------------------------------------------------------------------------
# Error envelope (unified per GIGI_LANG_SPEC.md §8a Q7)
# ---------------------------------------------------------------------------

class ErrorCategory(str, Enum):
    """
    Categories for GIGI Lang errors. Open enum — clients should pattern-match
    but treat unknown categories as forward-compatible additions, not failures.

    Categories map to GIGI's existing error variants where applicable:
        TRANSPORT    → TransportError
        INTEGRALITY  → IntegralityError
        QUANTUM      → QuantumError
    """
    TRANSLATION = "translation"
    GQL = "gql"
    TRANSPORT = "transport"
    INTEGRALITY = "integrality"
    QUANTUM = "quantum"
    AUTH = "auth"
    LICENSE_REQUIRED = "license_required"
    NOT_FOUND = "not_found"
    SERVER = "server"
    UNKNOWN = "unknown"


class GigiLangError(GigiError):
    """
    Base exception for all GIGI Lang errors.

    Extends GigiError (from gigi.client) with a category and structured details
    so callers can dispatch on category without parsing message strings.

    Attributes:
        category: ErrorCategory enum value identifying the error kind.
        details:  Dict of category-specific details (e.g., for TRANSLATION,
                  may include 'candidates' list of alternative GQL queries).
    """

    def __init__(self, message: str, category: ErrorCategory, **details: Any):
        super().__init__(message)
        self.category = category
        self.details = details


class TranslationError(GigiLangError):
    """
    Raised when prompt → GQL translation fails or is too ambiguous to proceed.

    For genuinely ambiguous prompts where the translator produced viable
    candidates but couldn't pick one with sufficient confidence, the
    `candidates` field surfaces them so the caller can present a choice.

    Attributes:
        candidates: List of candidate GQL queries the translator considered
                    but couldn't choose between. Empty if translation failed
                    outright (no viable parse).
    """

    def __init__(self, message: str, candidates: Optional[List[GQLQuery]] = None):
        super().__init__(
            message,
            ErrorCategory.TRANSLATION,
            candidates=candidates or [],
        )


class GQLExecutionError(GigiLangError):
    """Raised when a GQL query fails at GIGI's execution layer (post-translation)."""

    def __init__(self, message: str, gql: GQLQuery, status_code: Optional[int] = None):
        super().__init__(
            message,
            ErrorCategory.GQL,
            gql=gql,
            status_code=status_code,
        )


class LicenseRequiredError(GigiLangError):
    """Raised when a query targets a gated (commercial-license-required) part of the schema."""

    def __init__(self, message: str, gated_field: str, license_info_url: str):
        super().__init__(
            message,
            ErrorCategory.LICENSE_REQUIRED,
            gated_field=gated_field,
            license_info_url=license_info_url,
        )


# ---------------------------------------------------------------------------
# Main client
# ---------------------------------------------------------------------------

class GigiLang:
    """
    GIGI Lang client — prompt-driven interface to a GIGI instance.

    Wraps an underlying GigiClient (REST/GQL transport) with a translation
    layer that compiles natural-language prompts to GQL and returns
    fiber-shaped responses.

    Three usage levels:

        # 1. High-level: prompt in, fiber response out
        lang = GigiLang(url="https://gigi-stream.fly.dev", api_key="...")
        response = lang.ask("show me the 10 nearest cities to Tokyo")

        # 2. Mid-level: inspect the translated GQL before execution
        result = lang.translate("show me the 10 nearest cities to Tokyo")
        print(result.gql)
        response = lang.execute(result.gql)

        # 3. Low-level: write GQL directly, bypass translation
        response = lang.query('{ cities(near: "Tokyo", limit: 10) { name } }')

    Args:
        url:            Base URL of the GIGI server (e.g. "https://gigi-stream.fly.dev").
        api_key:        Optional API key (for authenticated calls). Schema introspection
                        works without one; commercial-gated queries require it.
        translator:     Translator backend identifier. Default "claude" for v1.
                        Future values may include "claude-finetuned" (v2) or other LLMs.
        default_format: Default serialization format for responses ("dhoom" or "json").
                        DHOOM is the geometrically-native default; JSON for compatibility.
        client:         Optional pre-configured GigiClient to wrap. If not provided,
                        one is constructed from url + api_key.

    Raises:
        ValueError: If translator name is unrecognized or default_format is invalid.
    """

    def __init__(
        self,
        url: Optional[str] = None,
        api_key: Optional[str] = None,
        translator: str = "claude",
        default_format: str = "dhoom",
        client: Optional[GigiClient] = None,
    ):
        raise NotImplementedError(
            "GigiLang.__init__ is a v0.0 skeleton. "
            "See GIGI_LANG_SPEC.md (v0.1.1) for the contract this pins."
        )

    # ----- High-level: prompt → fiber response -----

    def ask(
        self,
        prompt: Prompt,
        context: Optional[Dict[str, Any]] = None,
        format: Optional[str] = None,
    ) -> FiberResponse:
        """
        Translate `prompt` to GQL, execute it, return the fiber response.

        The simplest entrypoint. Equivalent to:
            self.execute(self.translate(prompt, context=context).gql, format=format)
        but implemented as a single operation for efficiency
        (avoids round-tripping the GQL string through the caller).

        Args:
            prompt:  Natural-language prompt describing what to retrieve.
            context: Optional context dict (e.g., prior conversation turns,
                     preferred bundle names, time ranges). Format is
                     translator-implementation-specific.
            format:  Override the default response format ("dhoom" or "json").

        Returns:
            FiberResponse with the data, fiber metadata, and format.

        Raises:
            TranslationError:    If the prompt cannot be translated unambiguously.
            GQLExecutionError:   If the translated GQL fails at execution.
            LicenseRequiredError: If the prompt resolves to a gated query.
            GigiLangError:       For other error categories (auth, server, etc.).
        """
        raise NotImplementedError()

    # ----- Mid-level: separate translation from execution -----

    def translate(
        self,
        prompt: Prompt,
        context: Optional[Dict[str, Any]] = None,
    ) -> TranslationResult:
        """
        Translate a natural-language prompt to GQL without executing it.

        Useful for audit, refinement, or showing the user what query will run
        before they commit to it.

        Args:
            prompt:  Natural-language prompt.
            context: Optional translator context.

        Returns:
            TranslationResult with the GQL query, alternates (if any),
            confidence, and optional notes.

        Raises:
            TranslationError: If the translator cannot produce any viable
                              GQL (e.g., prompt references types that don't exist).
        """
        raise NotImplementedError()

    def execute(
        self,
        gql: GQLQuery,
        format: Optional[str] = None,
    ) -> FiberResponse:
        """
        Execute a GQL query (typically obtained from `translate()`) against GIGI.

        Semantically identical to `query()` — the distinction is intent.
        Use `execute()` when the GQL came from `translate()`; use `query()`
        when the caller wrote the GQL directly.

        Args:
            gql:    The GQL query string to execute.
            format: Override the default response format ("dhoom" or "json").

        Returns:
            FiberResponse with the data, fiber metadata, and format.

        Raises:
            GQLExecutionError:    If GIGI rejects or fails to execute the GQL.
            LicenseRequiredError: If the query targets a gated capability.
            GigiLangError:        For other error categories.
        """
        raise NotImplementedError()

    # ----- Low-level: caller-written GQL -----

    def query(
        self,
        gql: GQLQuery,
        format: Optional[str] = None,
    ) -> FiberResponse:
        """
        Execute caller-authored GQL directly. Bypasses translation entirely.

        For callers who already know GQL and want to skip the prompt layer.
        Implementation-identical to `execute()`; kept as a separate method
        for API clarity (intent: caller-authored vs. translator-produced).

        Args:
            gql:    The GQL query string written by the caller.
            format: Override the default response format ("dhoom" or "json").

        Returns:
            FiberResponse with the data, fiber metadata, and format.

        Raises:
            GQLExecutionError:    If GIGI rejects or fails to execute the GQL.
            LicenseRequiredError: If the query targets a gated capability.
            GigiLangError:        For other error categories.
        """
        raise NotImplementedError()

    # ----- Schema introspection -----

    def schema(
        self,
        format: str = "sdl",
        version: Optional[str] = None,
    ) -> str:
        """
        Fetch GIGI's GraphQL schema.

        Cached per session; refetched on cache invalidation (ETag-based).

        Args:
            format:  "sdl" (default), "json" (GraphQL introspection JSON),
                     or "dhoom" (fiber-shaped). See
                     GIGI_SCHEMA_INTROSPECTION_SPEC.md §3 for details.
            version: Optional version pin (e.g., "v1", "v1.3"). If omitted,
                     fetches the latest. Pinning provides stability against
                     additive schema changes.

        Returns:
            Schema as a string in the requested format.

        Raises:
            GigiLangError: With ErrorCategory.SERVER if the endpoint is
                           unreachable or returns an unexpected status.
        """
        raise NotImplementedError()

    def schema_version(self) -> str:
        """
        Return the current schema version (e.g., "v1.3.2") without fetching the full schema.

        Uses a HEAD request — much cheaper than full schema fetch. Useful for
        cache validation and version-aware client logic.

        Returns:
            Version string in semver-ish format (vMAJOR.MINOR.PATCH).
        """
        raise NotImplementedError()
