# bohemia_graph

A Rust graph engine over the [Bohemia NER dataset](https://github.com/graphwright/ner-20260608) — a typed
knowledge graph extracted from "A Scandal in Bohemia" by Arthur Conan Doyle —
with Guile Scheme and Python FFI interfaces.

This project is a Rust learning exercise covering:

- Struct/enum definitions and `serde` deserialization from JSONL
- HashMap-indexed in-memory graph (no reference cycles — edges store IDs)
- BFS, transitive closure, and filtered edge queries
- `cdylib` shared library with C-ABI exports (`extern "C"`, `#[no_mangle]`)
- Calling Rust from Guile Scheme via `(system foreign)`
- Returning native Scheme values (strings, alists, lists) directly from Rust
  by calling the Guile C API (`scm_cons`, `scm_from_utf8_string`, …) and
  reinterpreting the result as SCM via `pointer->scm`
- Calling the same `cdylib` from Python via `ctypes` (zero new dependencies)

## Prerequisites

- Rust (1.70+) with `rustup`
- Guile Scheme 3.0 — `brew install guile` *(Guile demo only)*
- Python 3.10+ *(Python demo only — stdlib `ctypes` only, no pip installs)*
- The Bohemia JSONL dataset (see below)

## Data files

The four JSONL files are bundled in this repo (source:
[wware/ner-20260608](https://github.com/graphwright/ner-20260608)):

```
bohemia_entities.jsonl   — canonical entities with wiki links and aliases
bohemia_events.jsonl     — narrative events with participant lists
bohemia_moments.jsonl    — temporal anchors tied to events
bohemia_triplets.jsonl   — typed, directed statements (the graph edges)
```

`cargo run` looks for them in the current working directory (the repo root).
`guile query.scm` resolves them relative to the script's own directory, so it
works regardless of where you invoke it from.

## Build instructions

The Guile bottle from Homebrew on macOS is x86_64, so the shared library must
be cross-compiled for that target even on Apple Silicon:

```
rustup target add x86_64-apple-darwin
```

## Project layout

```
src/
  lib.rs        — crate root; re-exports all modules
  types.rs      — enums and structs (TruthStatus, EntityKind, Node, …)
  loader.rs     — JSONL → Vec<Node> via serde_json
  graph.rs      — in-memory graph with BFS / transitive closure / edge queries
  ffi.rs        — JSON-string FFI exports (char* return values)
  ffi_scm.rs    — native SCM FFI exports (Scm/usize return values, x86_64 only)
  guile_sys.rs  — extern "C" bindings to libguile + SCM immediate constants
  main.rs       — standalone CLI demo (cargo run)
build.rs        — links libguile-3.0 when targeting x86_64-apple-darwin
query.scm       — Guile Scheme demo using the native SCM API
bohemia_graph.py — Python ctypes wrapper (no pip installs required)
query.py        — Python demo mirroring query.scm
```

## Build

### Debug build (native, for `cargo run`)

```sh
cargo build
```

### Release build for Guile FFI (x86_64)

```sh
cargo build --release --target x86_64-apple-darwin
```

This produces `target/x86_64-apple-darwin/release/libbohemia_graph.dylib`.

## Run

### Rust CLI demo

```sh
cargo run
```

Loads the graph, prints some `describe` output, lists Holmes's outbound edges,
and runs a 2-hop BFS from Holmes.

Expected output (trimmed):

```
Loaded 713 nodes (319 entities/events/moments, 394 statements)

--- describe ---
Sherlock Holmes
Irene Adler

--- edges from Holmes (all) ---
  Sherlock Holmes -[AssociatedWith]-> Baker Street
  Sherlock Holmes -[Possesses]-> cigar case
  ...

--- BFS from Holmes, 2 hops ---
  layer 0: 1 nodes
    Sherlock Holmes
  layer 1: 28 nodes
    cigar case
    ...
```

### Guile Scheme demo

Build the x86_64 release first, then:

```sh
guile query.scm
```

The script uses the native SCM API (`graph_*_scm` exports).  Results are real
Scheme values — no JSON parsing required:

```scheme
;; describe → native string
(string? (graph-describe G "wiki:Sherlock_Holmes"))  ; ⇒ #t

;; edges-from → list of alists; use assq-ref directly
(let ((edges (graph-edges-from G "wiki:Sherlock_Holmes")))
  (for-each (lambda (e)
              (format #t "~a -> ~a~%"
                      (assq-ref e 'predicate)
                      (assq-ref e 'object-id)))
            edges))

;; bfs → list of layers, each a list of ID strings
(let ((layers (graph-bfs G '("wiki:Sherlock_Holmes") 2)))
  (format #t "layer 1 has ~a nodes~%" (length (cadr layers))))

;; node → full alist
(assq-ref (graph-node G "wiki:Irene_Adler") 'aliases)
;; ⇒ ("Irene Adler" "Mademoiselle Irene Adler" …)
```

### Python demo

The same `cdylib` is callable from Python with zero new dependencies — `ctypes`
is part of the standard library.  The wrapper module `bohemia_graph.py`
auto-discovers the built `.so` / `.dylib`.

```sh
# Build the release library (Linux / native arm64 macOS)
cargo build --release

python query.py
```

The wrapper exposes a `BohemiaGraph` class:

```python
from bohemia_graph import BohemiaGraph

with BohemiaGraph.find() as g:
    g.load("bohemia_entities.jsonl", "bohemia_events.jsonl",
           "bohemia_moments.jsonl", "bohemia_triplets.jsonl")

    # describe → plain string
    print(g.describe("wiki:Sherlock_Holmes"))   # "Sherlock Holmes"

    # edges_from → list of dicts (JSON-decoded StatementNode)
    for edge in g.edges_from("wiki:Sherlock_Holmes"):
        print(edge["predicate"], "->", edge["object_id"])

    # bfs → list of layers (each a sorted list of canonical ID strings)
    layers = g.bfs(["wiki:Sherlock_Holmes"], max_hops=2)
    print(f"layer 1: {len(layers[1])} nodes")

    # get → full node dict; node_kind == "entity" or "statement"
    node = g.get("wiki:Irene_Adler")
    print(node["aliases"])
```

The wrapper searches these paths for the library (in order):
`target/release`, `target/x86_64-apple-darwin/release`,
`target/aarch64-apple-darwin/release`, `target/debug`, `.`
— first relative to `bohemia_graph.py` itself, then relative to the current
working directory.  Pass `lib_path=` to `BohemiaGraph()` to override.

## Graph model

Nodes are either **entities** (persons, locations, objects, events, moments)
or **statements** (typed, directed edges with provenance). Statements are
first-class nodes so they can themselves be the subject or object of other
statements.

| Node type     | Example ID                       |
|---------------|----------------------------------|
| Entity/Person | `wiki:Sherlock_Holmes`           |
| Entity/Place  | `place:baker_street`             |
| Event         | `sib:event:watson_visits_holmes` |
| Moment        | `sib:moment:night_of_20_march`   |
| Statement     | `stmt:wiki:Sherlock_Holmes:Possesses:obj:cigar_case` |

Wiki URLs (`https://bakerstreet.fandom.com/wiki/…`) are automatically
canonicalized to `wiki:<slug>` on lookup.

## FFI API

The library exports two families of functions.

### Lifecycle (shared by both families)

```c
OpaqueGraph* graph_new();
int          graph_load(OpaqueGraph*, const char* entities, const char* events,
                        const char* moments, const char* triplets,
                        int sentence_cutoff);   // cutoff < 0 = no cutoff
void         graph_destroy(OpaqueGraph*);
int          graph_node_count(const OpaqueGraph*);
```

### JSON family (`ffi.rs`) — return `char*`

All returned strings are null-terminated UTF-8 heap allocations; the caller
must free them with `graph_free_str`.

```c
void  graph_free_str(char*);
char* graph_get(const OpaqueGraph*, const char* id);
char* graph_describe(const OpaqueGraph*, const char* id);
char* graph_edges_from(const OpaqueGraph*, const char* id,
                       const char* pred_type,   // NULL = any
                       const char* truth);       // NULL = any; e.g. "asserted_true"
char* graph_edges_to(const OpaqueGraph*, const char* id,
                     const char* pred_type, const char* truth);
char* graph_bfs(const OpaqueGraph*,
                const char* seeds_json,  // JSON array of ID strings
                int max_hops,
                const char* truth_json); // JSON array of truth values, or NULL
char* graph_transitive_closure(const OpaqueGraph*,
                               const char* start, const char* predicate);
```

### SCM family (`ffi_scm.rs`) — return native Scheme values

Available on x86_64 only (where libguile is linked).  Return type is `SCM`
(`uintptr_t`), which Guile reinterprets via `pointer->scm`.  No allocation to
free — values are owned by the Guile GC once returned.

Entities come back as alists with symbol keys:
`node-kind`, `id`, `display-name`, `kind`, `wiki-url`, `aliases`.

Statements come back as alists with symbol keys:
`id`, `predicate`, `subject-id`, `object-id`, `truth-status`, `story-id`,
`paragraph-index`, `sentence-ids`, `extraction-confidence`,
`asserting-narrator-id`.

```c
SCM graph_describe_scm(const OpaqueGraph*, const char* id);
SCM graph_node_scm(const OpaqueGraph*, const char* id);
SCM graph_edges_from_scm(const OpaqueGraph*, const char* id,
                         const char* pred_type, const char* truth);
SCM graph_edges_to_scm(const OpaqueGraph*, const char* id,
                       const char* pred_type, const char* truth);
SCM graph_bfs_scm(const OpaqueGraph*,
                  const char* seeds_json, int max_hops,
                  const char* truth_json);
SCM graph_transitive_closure_scm(const OpaqueGraph*,
                                 const char* start, const char* predicate);
SCM graph_all_ids_scm(const OpaqueGraph*);
SCM graph_canonicalize_scm(const char* id);
```

### How `pointer->scm` works

Guile's `(system foreign)` wraps any `'*` return value in a pointer object.
`pointer->scm` unsafely casts the pointer word to an SCM — which is exactly
right here because the Rust function returned a `uintptr_t` that already
encodes a valid SCM value built with `scm_cons`, `scm_from_utf8_string`, etc.
