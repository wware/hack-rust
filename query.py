#!/usr/bin/env python3
"""Python demo for the bohemia_graph knowledge-graph library.

Mirrors the Guile demo (query.scm) but uses the ctypes wrapper
(bohemia_graph.py) instead of Guile's ``(system foreign)`` machinery.

Run from the repo root after building the shared library::

    # Linux / native macOS arm64
    cargo build --release

    # macOS cross-compile (if using the x86_64 Guile bottle)
    cargo build --release --target x86_64-apple-darwin

    python query.py
"""

from pathlib import Path

from bohemia_graph import BohemiaGraph

HERE = Path(__file__).parent.resolve()

HOLMES = "wiki:Sherlock_Holmes"
ADLER  = "wiki:Irene_Adler"

with BohemiaGraph.find() as g:
    g.load(
        entities=HERE / "bohemia_entities.jsonl",
        events=HERE   / "bohemia_events.jsonl",
        moments=HERE  / "bohemia_moments.jsonl",
        triplets=HERE / "bohemia_triplets.jsonl",
    )
    print(f"Loaded {g.node_count()} nodes\n")

    # -- describe returns a plain string ------------------------------------
    print("--- describe (plain strings) ---")
    print(g.describe(HOLMES))
    print(g.describe(ADLER))
    print()

    # -- edges_from returns a list of dicts ---------------------------------
    print("--- edges from Holmes (all) ---")
    edges = g.edges_from(HOLMES)
    print(f"{len(edges)} edges returned")
    for edge in edges:
        print(f"  {edge['subject_id']} -[{edge['predicate']}]-> {edge['object_id']}")
    print()

    # -- edges_to with predicate filter ------------------------------------
    print("--- edges INTO Holmes (predicate='Investigates') ---")
    for edge in g.edges_to(HOLMES, pred_type="Investigates"):
        print(f"  {edge['subject_id']} -[{edge['predicate']}]-> {edge['object_id']}")
    print()

    # -- bfs returns a list of layers --------------------------------------
    print("--- BFS from Holmes, 2 hops ---")
    layers = g.bfs([HOLMES], max_hops=2)
    for i, layer in enumerate(layers):
        print(f"  layer {i}: {len(layer)} nodes")
        for node_id in layer[:4]:
            print(f"    {node_id}")
        if len(layer) > 4:
            print(f"    ... and {len(layer) - 4} more")
    print()

    # -- transitive_closure ------------------------------------------------
    print("--- transitive closure via 'LocatedIn' from Baker Street ---")
    reachable = g.transitive_closure("place:baker_street", "LocatedIn")
    if reachable:
        for node_id in reachable:
            print(f"  {node_id}")
    else:
        print("  (none found)")
    print()

    # -- node dict ---------------------------------------------------------
    print("--- Irene Adler node (dict) ---")
    node = g.get(ADLER)
    if node:
        for key, val in node.items():
            print(f"  {key}: {val!r}")
    print()

print("Done.")
