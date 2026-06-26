/// FFI layer for Guile Scheme (and any C caller).
///
/// All functions follow a simple contract:
///   - Graph is an opaque heap pointer (*mut OpaqueGraph).
///   - Strings cross the boundary as null-terminated UTF-8 C strings.
///   - Functions that return strings allocate with Box<CString>; the caller
///     must release them with graph_free_str().
///   - NULL input pointers return NULL / -1 / false as appropriate.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::Path;

use crate::graph::Graph;
use crate::loader::load_graph_data;
use crate::types::{EdgeFilter, TruthStatus};

// Opaque wrapper so C callers cannot dereference the pointer.
pub struct OpaqueGraph(pub Graph);

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Allocate an empty graph.  Returns NULL on allocation failure (never in practice).
#[unsafe(no_mangle)]
pub extern "C" fn graph_new() -> *mut OpaqueGraph {
    let g = Graph::new(vec![]);
    Box::into_raw(Box::new(OpaqueGraph(g)))
}

/// Load JSONL data files into the graph.
/// Paths must be valid UTF-8, null-terminated C strings.
/// `sentence_cutoff` < 0 means no cutoff.
/// Returns 0 on success, -1 on error (error printed to stderr).
#[unsafe(no_mangle)]
pub extern "C" fn graph_load(
    handle: *mut OpaqueGraph,
    entities: *const c_char,
    events: *const c_char,
    moments: *const c_char,
    triplets: *const c_char,
    sentence_cutoff: c_int,
) -> c_int {
    if handle.is_null() {
        return -1;
    }
    let to_path = |p: *const c_char| -> Option<String> {
        if p.is_null() {
            None
        } else {
            unsafe { CStr::from_ptr(p) }
                .to_str()
                .ok()
                .map(|s| s.to_string())
        }
    };
    let (Some(ep), Some(evp), Some(mp), Some(tp)) = (
        to_path(entities),
        to_path(events),
        to_path(moments),
        to_path(triplets),
    ) else {
        eprintln!("[ffi] graph_load: null path argument");
        return -1;
    };

    let cutoff = if sentence_cutoff < 0 {
        None
    } else {
        Some(sentence_cutoff as u32)
    };

    match load_graph_data(
        Path::new(&ep),
        Path::new(&evp),
        Path::new(&mp),
        Path::new(&tp),
        cutoff,
    ) {
        Ok(data) => {
            let g = Graph::new(data.nodes);
            unsafe { (*handle).0 = g };
            0
        }
        Err(e) => {
            eprintln!("[ffi] graph_load error: {e}");
            -1
        }
    }
}

/// Free a graph allocated by graph_new().
#[unsafe(no_mangle)]
pub extern "C" fn graph_destroy(handle: *mut OpaqueGraph) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

// ---------------------------------------------------------------------------
// String result helpers
// ---------------------------------------------------------------------------

/// Free a C string returned by any graph_* function.
#[unsafe(no_mangle)]
pub extern "C" fn graph_free_str(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)) };
    }
}

fn to_c_string(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Queries — all return JSON strings (caller must graph_free_str the result)
// ---------------------------------------------------------------------------

/// Look up a node by id.  Returns a JSON object, or null if not found.
#[unsafe(no_mangle)]
pub extern "C" fn graph_get(handle: *const OpaqueGraph, id: *const c_char) -> *mut c_char {
    let (Some(graph), Some(id_str)) = (
        unsafe { handle.as_ref() }.map(|h| &h.0),
        unsafe { id.as_ref() }
            .and_then(|_| unsafe { CStr::from_ptr(id) }.to_str().ok()),
    ) else {
        return std::ptr::null_mut();
    };

    match graph.get(id_str) {
        None => std::ptr::null_mut(),
        Some(node) => match serde_json::to_string(node) {
            Ok(json) => to_c_string(json),
            Err(_) => std::ptr::null_mut(),
        },
    }
}

/// Describe a node in human-readable form.
#[unsafe(no_mangle)]
pub extern "C" fn graph_describe(
    handle: *const OpaqueGraph,
    id: *const c_char,
) -> *mut c_char {
    let (Some(graph), Some(id_str)) = (
        unsafe { handle.as_ref() }.map(|h| &h.0),
        unsafe { id.as_ref() }
            .and_then(|_| unsafe { CStr::from_ptr(id) }.to_str().ok()),
    ) else {
        return std::ptr::null_mut();
    };
    to_c_string(graph.describe(id_str))
}

/// Returns a JSON array of statement nodes whose subject == id.
/// `pred_type` and `truth` may be null to mean "no filter".
#[unsafe(no_mangle)]
pub extern "C" fn graph_edges_from(
    handle: *const OpaqueGraph,
    id: *const c_char,
    pred_type: *const c_char,
    truth: *const c_char,
) -> *mut c_char {
    edges_query(handle, id, pred_type, truth, true)
}

/// Returns a JSON array of statement nodes whose object == id.
#[unsafe(no_mangle)]
pub extern "C" fn graph_edges_to(
    handle: *const OpaqueGraph,
    id: *const c_char,
    pred_type: *const c_char,
    truth: *const c_char,
) -> *mut c_char {
    edges_query(handle, id, pred_type, truth, false)
}

fn edges_query(
    handle: *const OpaqueGraph,
    id: *const c_char,
    pred_type: *const c_char,
    truth: *const c_char,
    outbound: bool,
) -> *mut c_char {
    let Some(graph) = unsafe { handle.as_ref() }.map(|h| &h.0) else {
        return std::ptr::null_mut();
    };
    let Some(id_str) = (unsafe { id.as_ref() })
        .and_then(|_| unsafe { CStr::from_ptr(id) }.to_str().ok())
    else {
        return std::ptr::null_mut();
    };

    let pred = if pred_type.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(pred_type) }
            .to_str()
            .ok()
            .map(|s| s.to_string())
    };

    let truth_status = if truth.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(truth) }
            .to_str()
            .ok()
            .and_then(parse_truth_status)
    };

    let filter = EdgeFilter {
        pred_type: pred,
        truth: truth_status,
    };

    let stmts: Vec<_> = if outbound {
        graph.edges_from(id_str, &filter)
    } else {
        graph.edges_to(id_str, &filter)
    };

    match serde_json::to_string(&stmts) {
        Ok(json) => to_c_string(json),
        Err(_) => std::ptr::null_mut(),
    }
}

/// BFS from a JSON array of seed IDs.
/// `seeds_json` example: `["wiki:Sherlock_Holmes","wiki:Irene_Adler"]`
/// `truth_json` may be null (defaults to asserted_true only).
/// Returns JSON object: `{"layers": [[...],[...],...]}`
#[unsafe(no_mangle)]
pub extern "C" fn graph_bfs(
    handle: *const OpaqueGraph,
    seeds_json: *const c_char,
    max_hops: c_int,
    truth_json: *const c_char,
) -> *mut c_char {
    let Some(graph) = unsafe { handle.as_ref() }.map(|h| &h.0) else {
        return std::ptr::null_mut();
    };
    let Some(seeds_str) = (unsafe { seeds_json.as_ref() })
        .and_then(|_| unsafe { CStr::from_ptr(seeds_json) }.to_str().ok())
    else {
        return std::ptr::null_mut();
    };

    let Ok(seed_ids): Result<Vec<String>, _> = serde_json::from_str(seeds_str) else {
        eprintln!("[ffi] graph_bfs: invalid seeds JSON");
        return std::ptr::null_mut();
    };

    let truth_values: Option<Vec<TruthStatus>> = if truth_json.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(truth_json) }
            .to_str()
            .ok()
            .and_then(|s| serde_json::from_str(s).ok())
    };

    let hops = if max_hops < 0 { 2 } else { max_hops as usize };
    let seed_refs: Vec<&str> = seed_ids.iter().map(|s| s.as_str()).collect();
    let layers = graph.bfs(
        &seed_refs,
        hops,
        truth_values.as_deref(),
    );

    let layers_vec: Vec<Vec<&str>> = layers
        .iter()
        .map(|set| {
            let mut v: Vec<&str> = set.iter().map(|s| s.as_str()).collect();
            v.sort();
            v
        })
        .collect();

    match serde_json::to_string(&serde_json::json!({"layers": layers_vec})) {
        Ok(json) => to_c_string(json),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Transitive closure from `start` following edges with the given predicate name.
/// Returns a JSON array of reachable canonical IDs.
#[unsafe(no_mangle)]
pub extern "C" fn graph_transitive_closure(
    handle: *const OpaqueGraph,
    start: *const c_char,
    predicate: *const c_char,
) -> *mut c_char {
    let Some(graph) = unsafe { handle.as_ref() }.map(|h| &h.0) else {
        return std::ptr::null_mut();
    };
    let (Some(start_str), Some(pred_str)) = (
        unsafe { start.as_ref() }
            .and_then(|_| unsafe { CStr::from_ptr(start) }.to_str().ok()),
        unsafe { predicate.as_ref() }
            .and_then(|_| unsafe { CStr::from_ptr(predicate) }.to_str().ok()),
    ) else {
        return std::ptr::null_mut();
    };

    let mut reachable: Vec<String> = graph
        .transitive_closure(start_str, pred_str)
        .into_iter()
        .collect();
    reachable.sort();

    match serde_json::to_string(&reachable) {
        Ok(json) => to_c_string(json),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Number of nodes in the graph.
#[unsafe(no_mangle)]
pub extern "C" fn graph_node_count(handle: *const OpaqueGraph) -> c_int {
    unsafe { handle.as_ref() }
        .map(|h| h.0.by_id.len() as c_int)
        .unwrap_or(-1)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_truth_status(s: &str) -> Option<TruthStatus> {
    match s {
        "asserted_true" => Some(TruthStatus::AssertedTrue),
        "asserted_false" => Some(TruthStatus::AssertedFalse),
        "hypothetical" => Some(TruthStatus::Hypothetical),
        "disputed" => Some(TruthStatus::Disputed),
        "retracted" => Some(TruthStatus::Retracted),
        _ => None,
    }
}
