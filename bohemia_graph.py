"""
Python ctypes wrapper for the bohemia_graph shared library.

Loads ``libbohemia_graph.so`` / ``libbohemia_graph.dylib`` and exposes the C
ABI (``ffi.rs``) as a Pythonic ``BohemiaGraph`` class — no compilation step
beyond the normal Rust build, no new dependencies.

Quick start
-----------
::

    from bohemia_graph import BohemiaGraph

    with BohemiaGraph.find() as g:
        g.load("bohemia_entities.jsonl", "bohemia_events.jsonl",
               "bohemia_moments.jsonl", "bohemia_triplets.jsonl")

        print(g.describe("wiki:Sherlock_Holmes"))

        for edge in g.edges_from("wiki:Sherlock_Holmes"):
            print(edge["predicate"], "->", edge["object_id"])

        layers = g.bfs(["wiki:Sherlock_Holmes"], max_hops=2)
        print(f"2-hop neighbourhood: {sum(len(l) for l in layers)} nodes")

Build the shared library first (see README for platform details)::

    cargo build --release          # Linux / native macOS arm64
    # or cross-compile for Guile on macOS:
    # cargo build --release --target x86_64-apple-darwin
"""

from __future__ import annotations

import ctypes
import json
import os
import sys
from pathlib import Path
from typing import Optional, Union

# ---------------------------------------------------------------------------
# Library discovery
# ---------------------------------------------------------------------------

_LIB_NAMES: dict[str, list[str]] = {
    "linux":  ["libbohemia_graph.so"],
    "darwin": ["libbohemia_graph.dylib"],
    "win32":  ["bohemia_graph.dll"],
}

# Candidate subdirectories searched relative to the script root and CWD.
_SEARCH_DIRS = [
    "target/release",
    "target/x86_64-apple-darwin/release",
    "target/aarch64-apple-darwin/release",
    "target/debug",
    "target/x86_64-apple-darwin/debug",
    ".",
]


def _find_lib(lib_path: Optional[str] = None) -> str:
    """Return the path to the shared library, raising if not found."""
    if lib_path:
        return lib_path

    platform = sys.platform
    names = _LIB_NAMES.get(platform, [])
    if not names:
        raise RuntimeError(
            f"Platform {platform!r} is not supported by this wrapper. "
            "Pass lib_path= explicitly."
        )

    roots = [Path(__file__).parent.resolve(), Path.cwd().resolve()]
    for root in roots:
        for subdir in _SEARCH_DIRS:
            for name in names:
                candidate = root / subdir / name
                if candidate.exists():
                    return str(candidate)

    raise FileNotFoundError(
        f"Could not locate the bohemia_graph shared library ({names}). "
        "Build it with `cargo build --release` and retry, or pass "
        "lib_path= explicitly to BohemiaGraph()."
    )


# ---------------------------------------------------------------------------
# Low-level ctypes shim
# ---------------------------------------------------------------------------

class _Lib:
    """Thin, type-annotated ctypes wrapper around the C ABI exported by ffi.rs."""

    def __init__(self, path: str) -> None:
        self._lib = ctypes.CDLL(path)
        self._declare()

    # -- ABI declarations ----------------------------------------------------

    def _declare(self) -> None:
        lib = self._lib

        # Lifecycle
        lib.graph_new.argtypes = []
        lib.graph_new.restype = ctypes.c_void_p

        lib.graph_load.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # entities path
            ctypes.c_char_p,  # events path
            ctypes.c_char_p,  # moments path
            ctypes.c_char_p,  # triplets path
            ctypes.c_int,     # sentence_cutoff  (< 0 = no cutoff)
        ]
        lib.graph_load.restype = ctypes.c_int

        lib.graph_destroy.argtypes = [ctypes.c_void_p]
        lib.graph_destroy.restype = None

        lib.graph_node_count.argtypes = [ctypes.c_void_p]
        lib.graph_node_count.restype = ctypes.c_int

        # String management
        lib.graph_free_str.argtypes = [ctypes.c_void_p]
        lib.graph_free_str.restype = None

        # Queries — all return a heap-allocated char* (freed by graph_free_str)
        lib.graph_get.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        lib.graph_get.restype = ctypes.c_void_p

        lib.graph_describe.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        lib.graph_describe.restype = ctypes.c_void_p

        lib.graph_edges_from.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # id
            ctypes.c_char_p,  # pred_type  (NULL = any)
            ctypes.c_char_p,  # truth      (NULL = any)
        ]
        lib.graph_edges_from.restype = ctypes.c_void_p

        lib.graph_edges_to.argtypes = [
            ctypes.c_void_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
            ctypes.c_char_p,
        ]
        lib.graph_edges_to.restype = ctypes.c_void_p

        lib.graph_bfs.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # seeds_json  (JSON array of ID strings)
            ctypes.c_int,     # max_hops
            ctypes.c_char_p,  # truth_json  (JSON array of truth values, or NULL)
        ]
        lib.graph_bfs.restype = ctypes.c_void_p

        lib.graph_transitive_closure.argtypes = [
            ctypes.c_void_p,  # handle
            ctypes.c_char_p,  # start
            ctypes.c_char_p,  # predicate
        ]
        lib.graph_transitive_closure.restype = ctypes.c_void_p

    # -- Helpers -------------------------------------------------------------

    def _consume_str(self, ptr: Optional[int]) -> Optional[str]:
        """Read and free a C string returned by the library; return a Python str."""
        if ptr is None or ptr == 0:
            return None
        try:
            result = ctypes.string_at(ptr).decode("utf-8")
        finally:
            self._lib.graph_free_str(ptr)
        return result

    @staticmethod
    def _enc(s: Optional[str]) -> Optional[bytes]:
        return s.encode("utf-8") if s is not None else None

    # -- Lifecycle wrappers --------------------------------------------------

    def graph_new(self) -> int:
        h = self._lib.graph_new()
        if not h:
            raise MemoryError("graph_new() returned NULL")
        return h

    def graph_load(
        self,
        handle: int,
        entities: str,
        events: str,
        moments: str,
        triplets: str,
        sentence_cutoff: int,
    ) -> int:
        return self._lib.graph_load(
            handle,
            entities.encode(),
            events.encode(),
            moments.encode(),
            triplets.encode(),
            sentence_cutoff,
        )

    def graph_destroy(self, handle: int) -> None:
        self._lib.graph_destroy(handle)

    def graph_node_count(self, handle: int) -> int:
        return self._lib.graph_node_count(handle)

    # -- Query wrappers ------------------------------------------------------

    def graph_get(self, handle: int, node_id: str) -> Optional[str]:
        return self._consume_str(self._lib.graph_get(handle, node_id.encode()))

    def graph_describe(self, handle: int, node_id: str) -> Optional[str]:
        return self._consume_str(self._lib.graph_describe(handle, node_id.encode()))

    def graph_edges_from(
        self,
        handle: int,
        node_id: str,
        pred_type: Optional[str],
        truth: Optional[str],
    ) -> Optional[str]:
        return self._consume_str(
            self._lib.graph_edges_from(
                handle, node_id.encode(), self._enc(pred_type), self._enc(truth)
            )
        )

    def graph_edges_to(
        self,
        handle: int,
        node_id: str,
        pred_type: Optional[str],
        truth: Optional[str],
    ) -> Optional[str]:
        return self._consume_str(
            self._lib.graph_edges_to(
                handle, node_id.encode(), self._enc(pred_type), self._enc(truth)
            )
        )

    def graph_bfs(
        self,
        handle: int,
        seeds_json: str,
        max_hops: int,
        truth_json: Optional[str],
    ) -> Optional[str]:
        return self._consume_str(
            self._lib.graph_bfs(
                handle, seeds_json.encode(), max_hops, self._enc(truth_json)
            )
        )

    def graph_transitive_closure(
        self, handle: int, start: str, predicate: str
    ) -> Optional[str]:
        return self._consume_str(
            self._lib.graph_transitive_closure(
                handle, start.encode(), predicate.encode()
            )
        )


# ---------------------------------------------------------------------------
# High-level API
# ---------------------------------------------------------------------------

class BohemiaGraph:
    """
    Pythonic wrapper around the bohemia_graph knowledge-graph library.

    Supports use as a context manager for automatic resource cleanup::

        with BohemiaGraph.find() as g:
            g.load(...)
            ...

    All query methods that accept optional ``pred_type`` / ``truth`` filters
    pass ``None`` as a C NULL, meaning "no filter".

    Truth-status strings recognised by the library:
    ``"asserted_true"``, ``"asserted_false"``, ``"hypothetical"``,
    ``"disputed"``, ``"retracted"``.
    """

    def __init__(self, lib_path: Optional[str] = None) -> None:
        """
        Parameters
        ----------
        lib_path:
            Explicit path to the shared library.  When omitted, common build
            output directories are searched automatically (see :func:`_find_lib`).
        """
        resolved = _find_lib(lib_path)
        self._lib = _Lib(resolved)
        self._handle: Optional[int] = self._lib.graph_new()

    # -- Context manager -----------------------------------------------------

    def __enter__(self) -> "BohemiaGraph":
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    def close(self) -> None:
        """Release the graph's memory.  Safe to call more than once."""
        if self._handle is not None:
            self._lib.graph_destroy(self._handle)
            self._handle = None

    # -- Data loading --------------------------------------------------------

    def load(
        self,
        entities: Union[str, os.PathLike],
        events: Union[str, os.PathLike],
        moments: Union[str, os.PathLike],
        triplets: Union[str, os.PathLike],
        sentence_cutoff: int = -1,
    ) -> "BohemiaGraph":
        """
        Load the four JSONL data files into the graph.

        Parameters
        ----------
        entities:
            Path to ``bohemia_entities.jsonl``.
        events:
            Path to ``bohemia_events.jsonl``.
        moments:
            Path to ``bohemia_moments.jsonl``.
        triplets:
            Path to ``bohemia_triplets.jsonl``.
        sentence_cutoff:
            Only load statements whose ``sentence_ids`` contain an ID
            ``<= sentence_cutoff``.  Pass ``-1`` (default) to disable.

        Returns
        -------
        self — so you can chain: ``BohemiaGraph.find().load(...)``.
        """
        assert self._handle is not None, "Graph has been closed"
        rc = self._lib.graph_load(
            self._handle,
            str(entities), str(events), str(moments), str(triplets),
            sentence_cutoff,
        )
        if rc != 0:
            raise RuntimeError(
                "graph_load() returned non-zero; check stderr for details"
            )
        return self

    # -- Queries -------------------------------------------------------------

    def node_count(self) -> int:
        """Total number of nodes (entities + statements) in the graph."""
        assert self._handle is not None, "Graph has been closed"
        return self._lib.graph_node_count(self._handle)

    def get(self, node_id: str) -> Optional[dict]:
        """
        Look up a node by ID.

        Returns a dict (JSON-decoded) or ``None`` if the ID is not found.
        Entity nodes carry a ``"node_kind": "entity"`` key; statement nodes
        carry ``"node_kind": "statement"``.
        Wiki URLs are automatically canonicalised (``wiki:<slug>``).
        """
        assert self._handle is not None, "Graph has been closed"
        raw = self._lib.graph_get(self._handle, node_id)
        return json.loads(raw) if raw is not None else None

    def describe(self, node_id: str) -> Optional[str]:
        """
        Human-readable one-liner for a node.

        Entities → display name.
        Statements → ``"<subject> -[<predicate>]-> <object>"``.
        Returns ``None`` if the node is not found.
        """
        assert self._handle is not None, "Graph has been closed"
        return self._lib.graph_describe(self._handle, node_id)

    def edges_from(
        self,
        node_id: str,
        pred_type: Optional[str] = None,
        truth: Optional[str] = None,
    ) -> list[dict]:
        """
        Return all statement nodes whose **subject** is *node_id*.

        Parameters
        ----------
        pred_type:
            Case-insensitive predicate filter, e.g. ``"Possesses"``.
            ``None`` returns all predicates.
        truth:
            Truth-status filter, e.g. ``"asserted_true"``.
            ``None`` returns all truth statuses.

        Returns
        -------
        A list of statement dicts (JSON-decoded), each with keys:
        ``id``, ``predicate``, ``subject_id``, ``object_id``,
        ``truth_status``, ``story_id``, ``paragraph_index``,
        ``sentence_ids``, ``asserting_narrator_id``,
        ``extraction_confidence``.
        """
        assert self._handle is not None, "Graph has been closed"
        raw = self._lib.graph_edges_from(self._handle, node_id, pred_type, truth)
        return json.loads(raw) if raw is not None else []

    def edges_to(
        self,
        node_id: str,
        pred_type: Optional[str] = None,
        truth: Optional[str] = None,
    ) -> list[dict]:
        """
        Return all statement nodes whose **object** is *node_id*.

        Parameters and return value are the same as :meth:`edges_from`.
        """
        assert self._handle is not None, "Graph has been closed"
        raw = self._lib.graph_edges_to(self._handle, node_id, pred_type, truth)
        return json.loads(raw) if raw is not None else []

    def bfs(
        self,
        seeds: list[str],
        max_hops: int = 2,
        truth_values: Optional[list[str]] = None,
    ) -> list[list[str]]:
        """
        Breadth-first search from *seeds*.

        Parameters
        ----------
        seeds:
            Starting node IDs, e.g. ``["wiki:Sherlock_Holmes"]``.
        max_hops:
            Maximum BFS depth.  Default 2.
        truth_values:
            List of truth-status strings to traverse.  ``None`` (default)
            restricts traversal to ``"asserted_true"`` edges.

        Returns
        -------
        A list of *max_hops + 1* layers; ``layers[0]`` contains the seeds,
        ``layers[n]`` the nodes first reached at hop *n*.  Each layer is a
        list of canonical ID strings (sorted).
        """
        assert self._handle is not None, "Graph has been closed"
        seeds_json = json.dumps(seeds)
        truth_json = json.dumps(truth_values) if truth_values is not None else None
        raw = self._lib.graph_bfs(self._handle, seeds_json, max_hops, truth_json)
        if raw is None:
            return []
        obj = json.loads(raw)
        return obj.get("layers", [])

    def transitive_closure(self, start: str, predicate: str) -> list[str]:
        """
        Follow *predicate* edges transitively from *start*.

        Returns a sorted list of reachable canonical IDs (not including
        *start* itself).
        """
        assert self._handle is not None, "Graph has been closed"
        raw = self._lib.graph_transitive_closure(self._handle, start, predicate)
        return json.loads(raw) if raw is not None else []

    # -- Factory -------------------------------------------------------------

    @classmethod
    def find(cls, lib_path: Optional[str] = None) -> "BohemiaGraph":
        """
        Create a :class:`BohemiaGraph` with auto-detected library path.

        Equivalent to ``BohemiaGraph(lib_path)``, but reads more naturally
        as a constructor when *lib_path* is omitted::

            with BohemiaGraph.find() as g:
                ...
        """
        return cls(lib_path)
