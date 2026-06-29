/// Guile-native FFI layer: functions that return SCM values directly.
///
/// These are only compiled for x86_64 macOS (where libguile is available via
/// the Homebrew bottle).  Linux x86_64 and other targets skip this module to
/// avoid unresolvable libguile symbols in the shared library.
/// The Scheme side calls them via `(system foreign)` exactly like the JSON
/// variants, but gets back real Scheme alists and lists instead of strings.

#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
mod inner {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_int};

    use crate::ffi::OpaqueGraph;
    use crate::graph::canonicalize_id;
    use crate::guile_sys::guile::{self, Scm, SCM_BOOL_F, SCM_EOL};
    use crate::types::{EdgeFilter, Node, StatementNode, TruthStatus};

    // -- Conversion helpers -------------------------------------------------

    fn stmt_to_alist(stmt: &StatementNode) -> Scm {
        let sentence_ids = guile::scm_list_from_iter(
            stmt.sentence_ids.iter().map(|&s| guile::scm_u32(s)),
        );
        guile::scm_alist(&[
            ("id",                    guile::scm_str(&stmt.id)),
            ("predicate",             guile::scm_str(&stmt.predicate)),
            ("subject-id",            guile::scm_str(&stmt.subject_id)),
            ("object-id",             guile::scm_str(&stmt.object_id)),
            ("truth-status",          guile::scm_str(stmt.truth_status.as_str())),
            ("story-id",              guile::scm_str(&stmt.story_id)),
            ("paragraph-index",       guile::scm_u32(stmt.paragraph_index)),
            ("sentence-ids",          sentence_ids),
            ("extraction-confidence", guile::scm_f64(stmt.extraction_confidence)),
            ("asserting-narrator-id",
             guile::scm_opt_str(stmt.asserting_narrator_id.as_deref())),
        ])
    }

    fn node_to_alist(node: &Node) -> Scm {
        match node {
            Node::Entity(e) => {
                let aliases = guile::scm_list_from_iter(
                    e.aliases.iter().map(|a| guile::scm_str(a)),
                );
                guile::scm_alist(&[
                    ("node-kind",    guile::scm_str("entity")),
                    ("id",           guile::scm_str(&e.id)),
                    ("display-name", guile::scm_str(&e.display_name)),
                    ("kind",         guile::scm_str(&e.kind)),
                    ("wiki-url",     guile::scm_opt_str(e.wiki_url.as_deref())),
                    ("aliases",      aliases),
                ])
            }
            Node::Statement(s) => stmt_to_alist(s),
        }
    }

    fn parse_truth(p: *const c_char) -> Option<TruthStatus> {
        if p.is_null() {
            return None;
        }
        unsafe { CStr::from_ptr(p) }
            .to_str()
            .ok()
            .and_then(TruthStatus::parse)
    }

    fn c_str<'a>(p: *const c_char) -> Option<&'a str> {
        if p.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(p) }.to_str().ok()
        }
    }

    // -- Public FFI ---------------------------------------------------------

    /// Look up a node and return it as a Scheme alist, or #f if not found.
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_node_scm(
        handle: *const OpaqueGraph,
        id: *const c_char,
    ) -> Scm {
        let Some(graph) = (unsafe { handle.as_ref() }).map(|h| &h.0) else {
            return SCM_BOOL_F;
        };
        let Some(id_str) = c_str(id) else {
            return SCM_BOOL_F;
        };
        match graph.get(id_str) {
            None => SCM_BOOL_F,
            Some(node) => node_to_alist(node),
        }
    }

    /// Describe a node — returns a Scheme string, or #f if not found.
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_describe_scm(
        handle: *const OpaqueGraph,
        id: *const c_char,
    ) -> Scm {
        let Some(graph) = (unsafe { handle.as_ref() }).map(|h| &h.0) else {
            return SCM_BOOL_F;
        };
        let Some(id_str) = c_str(id) else {
            return SCM_BOOL_F;
        };
        guile::scm_str(&graph.describe(id_str))
    }

    /// Return outbound edges as a Scheme list of alists.
    /// pred and truth may be null (no filter).
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_edges_from_scm(
        handle: *const OpaqueGraph,
        id: *const c_char,
        pred: *const c_char,
        truth: *const c_char,
    ) -> Scm {
        edge_query_scm(handle, id, pred, truth, true)
    }

    /// Return inbound edges as a Scheme list of alists.
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_edges_to_scm(
        handle: *const OpaqueGraph,
        id: *const c_char,
        pred: *const c_char,
        truth: *const c_char,
    ) -> Scm {
        edge_query_scm(handle, id, pred, truth, false)
    }

    fn edge_query_scm(
        handle: *const OpaqueGraph,
        id: *const c_char,
        pred: *const c_char,
        truth: *const c_char,
        outbound: bool,
    ) -> Scm {
        let Some(graph) = (unsafe { handle.as_ref() }).map(|h| &h.0) else {
            return SCM_EOL;
        };
        let Some(id_str) = c_str(id) else {
            return SCM_EOL;
        };
        let filter = EdgeFilter {
            pred_type: c_str(pred).map(|s| s.to_string()),
            truth: parse_truth(truth),
        };
        let stmts = if outbound {
            graph.edges_from(id_str, &filter)
        } else {
            graph.edges_to(id_str, &filter)
        };
        guile::scm_list_from_iter(stmts.into_iter().map(stmt_to_alist))
    }

    /// BFS from a JSON array of seed IDs.
    /// Returns a Scheme list of layers; each layer is a list of canonical ID strings.
    /// truth_json may be null (defaults to asserted_true only).
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_bfs_scm(
        handle: *const OpaqueGraph,
        seeds_json: *const c_char,
        max_hops: c_int,
        truth_json: *const c_char,
    ) -> Scm {
        let Some(graph) = (unsafe { handle.as_ref() }).map(|h| &h.0) else {
            return SCM_EOL;
        };
        let Some(seeds_str) = c_str(seeds_json) else {
            return SCM_EOL;
        };
        let Ok(seed_ids): Result<Vec<String>, _> = serde_json::from_str(seeds_str) else {
            return SCM_EOL;
        };
        let truth_values: Option<Vec<TruthStatus>> = c_str(truth_json)
            .and_then(|s| serde_json::from_str(s).ok());

        let hops = if max_hops < 0 { 2 } else { max_hops as usize };
        let seed_refs: Vec<&str> = seed_ids.iter().map(|s| s.as_str()).collect();
        let layers = graph.bfs(&seed_refs, hops, truth_values.as_deref());

        guile::scm_list_from_iter(layers.into_iter().map(|layer| {
            let mut ids: Vec<&str> = layer.iter().map(|s| s.as_str()).collect();
            ids.sort();
            guile::scm_list_from_iter(ids.into_iter().map(guile::scm_str))
        }))
    }

    /// Transitive closure — returns a sorted Scheme list of canonical ID strings.
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_transitive_closure_scm(
        handle: *const OpaqueGraph,
        start: *const c_char,
        predicate: *const c_char,
    ) -> Scm {
        let Some(graph) = (unsafe { handle.as_ref() }).map(|h| &h.0) else {
            return SCM_EOL;
        };
        let (Some(start_str), Some(pred_str)) = (c_str(start), c_str(predicate)) else {
            return SCM_EOL;
        };
        let mut ids: Vec<String> = graph
            .transitive_closure(start_str, pred_str)
            .into_iter()
            .collect();
        ids.sort();
        guile::scm_list_from_iter(ids.iter().map(|s| guile::scm_str(s)))
    }

    /// All canonical IDs in the graph as a sorted Scheme list of strings.
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_all_ids_scm(handle: *const OpaqueGraph) -> Scm {
        let Some(graph) = (unsafe { handle.as_ref() }).map(|h| &h.0) else {
            return SCM_EOL;
        };
        let mut ids: Vec<&str> = graph.by_id.keys().map(|s| s.as_str()).collect();
        ids.sort();
        guile::scm_list_from_iter(ids.into_iter().map(guile::scm_str))
    }

    /// Canonicalize an ID string — returns a Scheme string.
    #[unsafe(no_mangle)]
    pub extern "C" fn graph_canonicalize_scm(id: *const c_char) -> Scm {
        match c_str(id) {
            Some(s) => guile::scm_str(&canonicalize_id(s)),
            None => SCM_BOOL_F,
        }
    }
}

// Re-export so the symbols are visible at crate top level for x86_64 macOS.
#[cfg(all(target_arch = "x86_64", target_os = "macos"))]
pub use inner::*;
