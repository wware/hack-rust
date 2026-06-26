# bohemia_graph

A Rust graph engine over the [Bohemia NER dataset](../ner-20260608) — a typed
knowledge graph extracted from "A Scandal in Bohemia" by Arthur Conan Doyle —
with a Guile Scheme FFI interface.

This project is a Rust learning exercise covering:

- Struct/enum definitions and `serde` deserialization from JSONL
- HashMap-indexed in-memory graph (no reference cycles — edges store IDs)
- BFS, transitive closure, and filtered edge queries
- `cdylib` shared library with C-ABI exports (`extern "C"`, `#[no_mangle]`)
- Calling Rust from Guile Scheme via `(system foreign)`

## Prerequisites

- Rust (1.70+) with `rustup`
- Guile Scheme 3.0 — `brew install guile`
- The Bohemia JSONL dataset (see below)

## Data files

This project reads JSONL files produced by the
[ner-20260608](../ner-20260608) NER pipeline — a separate project that
extracted a typed knowledge graph from "A Scandal in Bohemia" using Claude.
Clone or copy that repo as a sibling directory so the relative path
`../ner-20260608/` resolves correctly:

```
parent/
  ner-20260608/          ← NER pipeline + generated JSONL files
    bohemia_entities.jsonl
    bohemia_events.jsonl
    bohemia_moments.jsonl
    bohemia_triplets.jsonl
    ...
  hack-rust/             ← this repo
```

Both `cargo run` and `guile query.scm` expect the files at that relative path.
If your data lives elsewhere, edit the `base` path at the top of `src/main.rs`
and `query.scm`.

## Build instructions

The Guile bottle from Homebrew on macOS is x86_64, so the shared library must
be cross-compiled for that target even on Apple Silicon:

```
rustup target add x86_64-apple-darwin
```

## Project layout

```
src/
  lib.rs       — crate root; re-exports all modules
  types.rs     — enums and structs (TruthStatus, EntityKind, Node, …)
  loader.rs    — JSONL → Vec<Node> via serde_json
  graph.rs     — in-memory graph with BFS / transitive closure / edge queries
  ffi.rs       — extern "C" exports for the cdylib target
  main.rs      — standalone CLI demo (cargo run)
query.scm      — Guile Scheme demo that dlopen's the shared library
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

The script loads the graph through the FFI and prints the same queries as the
Rust CLI, but driven entirely from Scheme. Results come back as JSON strings
that Scheme can parse with `(json->scm ...)` from the `(json)` module if
further processing is needed.

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

The shared library exports these C-compatible functions.  All string arguments
and return values are null-terminated UTF-8.  Returned strings must be freed
with `graph_free_str`.

```c
// Lifecycle
OpaqueGraph* graph_new();
int          graph_load(OpaqueGraph*, const char* entities, const char* events,
                        const char* moments, const char* triplets,
                        int sentence_cutoff);   // cutoff < 0 = no cutoff
void         graph_destroy(OpaqueGraph*);
void         graph_free_str(char*);

// Queries — return JSON strings
int   graph_node_count(const OpaqueGraph*);
char* graph_get(const OpaqueGraph*, const char* id);           // JSON object or NULL
char* graph_describe(const OpaqueGraph*, const char* id);      // human-readable string
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
                               const char* start,
                               const char* predicate);
```
