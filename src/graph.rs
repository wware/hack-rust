use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::{EdgeFilter, Node, StatementNode, TruthStatus};

const WIKI_PREFIX: &str = "https://bakerstreet.fandom.com/wiki/";

// ---------------------------------------------------------------------------
// Canonicalize IDs (mirrors Python _canonicalize_id)
// ---------------------------------------------------------------------------

pub fn canonicalize_id(id: &str) -> String {
    if let Some(slug) = id.strip_prefix(WIKI_PREFIX) {
        if !slug.is_empty() && !slug.contains('/') {
            return format!("wiki:{slug}");
        }
    }
    id.to_string()
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

pub struct Graph {
    // node storage, keyed by canonical ID
    pub by_id: HashMap<String, Node>,
    // forward index: entity_id -> list of statement IDs whose subject == entity
    pub out_edges: HashMap<String, Vec<String>>,
    // backward index: entity_id -> list of statement IDs whose object == entity
    pub in_edges: HashMap<String, Vec<String>>,
}

impl Graph {
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut by_id: HashMap<String, Node> = HashMap::new();
        let mut out_edges: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_edges: HashMap<String, Vec<String>> = HashMap::new();

        for node in nodes {
            let cid = canonicalize_id(node.id());
            if by_id.contains_key(&cid) {
                eprintln!("[warn] id collision: {cid}");
            }
            if let Node::Statement(ref stmt) = node {
                let subj = canonicalize_id(&stmt.subject_id);
                let obj = canonicalize_id(&stmt.object_id);
                out_edges.entry(subj).or_default().push(cid.clone());
                in_edges.entry(obj).or_default().push(cid.clone());
            }
            by_id.insert(cid, node);
        }

        Graph { by_id, out_edges, in_edges }
    }

    // -- Lookup -------------------------------------------------------------

    pub fn get(&self, id: &str) -> Option<&Node> {
        self.by_id.get(&canonicalize_id(id))
    }

    // -- Edge queries -------------------------------------------------------

    pub fn edges_from(&self, id: &str, filter: &EdgeFilter) -> Vec<&StatementNode> {
        let cid = canonicalize_id(id);
        self.out_edges
            .get(&cid)
            .map(|stmt_ids| {
                stmt_ids
                    .iter()
                    .filter_map(|sid| self.by_id.get(sid))
                    .filter_map(|n| if let Node::Statement(s) = n { Some(s) } else { None })
                    .filter(|s| self.matches_filter(s, filter))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn edges_to(&self, id: &str, filter: &EdgeFilter) -> Vec<&StatementNode> {
        let cid = canonicalize_id(id);
        self.in_edges
            .get(&cid)
            .map(|stmt_ids| {
                stmt_ids
                    .iter()
                    .filter_map(|sid| self.by_id.get(sid))
                    .filter_map(|n| if let Node::Statement(s) = n { Some(s) } else { None })
                    .filter(|s| self.matches_filter(s, filter))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn matches_filter(&self, stmt: &StatementNode, filter: &EdgeFilter) -> bool {
        if let Some(ref pred) = filter.pred_type {
            if !stmt.predicate.eq_ignore_ascii_case(pred) {
                return false;
            }
        }
        if let Some(ref truth) = filter.truth {
            if &stmt.truth_status != truth {
                return false;
            }
        }
        true
    }

    // -- BFS ----------------------------------------------------------------
    // Returns a Vec of layers; layer[0] = seeds, layer[n] = nodes reached at hop n.
    // By default only traverses asserted_true edges (mirrors Python default).

    pub fn bfs(
        &self,
        seeds: &[&str],
        max_hops: usize,
        truth_values: Option<&[TruthStatus]>,
    ) -> Vec<HashSet<String>> {
        let default_truth = [TruthStatus::AssertedTrue];
        let allowed: &[TruthStatus] = truth_values.unwrap_or(&default_truth);

        let mut layers: Vec<HashSet<String>> = vec![HashSet::new(); max_hops + 1];
        let mut visited: HashSet<String> = HashSet::new();

        for seed in seeds {
            let cid = canonicalize_id(seed);
            if self.by_id.contains_key(&cid) {
                layers[0].insert(cid.clone());
                visited.insert(cid);
            }
        }

        // BFS via a queue of (canonical_id, current_hop)
        let mut queue: VecDeque<(String, usize)> = layers[0]
            .iter()
            .map(|id| (id.clone(), 0))
            .collect();

        while let Some((current_id, hop)) = queue.pop_front() {
            if hop >= max_hops {
                continue;
            }

            let stmt_ids = self.out_edges.get(&current_id).cloned().unwrap_or_default();
            for stmt_id in stmt_ids {
                let Some(Node::Statement(stmt)) = self.by_id.get(&stmt_id) else {
                    continue;
                };
                if !allowed.contains(&stmt.truth_status) {
                    continue;
                }

                // Add the statement node itself
                if !visited.contains(&stmt_id) {
                    visited.insert(stmt_id.clone());
                    layers[hop + 1].insert(stmt_id.clone());
                    // Statements don't propagate BFS further
                }

                // Add the object entity
                let obj_id = canonicalize_id(&stmt.object_id);
                if !visited.contains(&obj_id) {
                    visited.insert(obj_id.clone());
                    layers[hop + 1].insert(obj_id.clone());
                    queue.push_back((obj_id, hop + 1));
                }
            }
        }

        layers
    }

    // -- Transitive closure -------------------------------------------------
    // Follow edges of a given predicate name until no new nodes are found.

    pub fn transitive_closure(&self, start: &str, predicate: &str) -> HashSet<String> {
        let mut reachable: HashSet<String> = HashSet::new();
        let mut frontier: VecDeque<String> = VecDeque::new();
        frontier.push_back(canonicalize_id(start));

        while let Some(current) = frontier.pop_front() {
            let filter = EdgeFilter {
                pred_type: Some(predicate.to_string()),
                truth: None,
            };
            for stmt in self.edges_from(&current, &filter) {
                let obj = canonicalize_id(&stmt.object_id);
                if reachable.insert(obj.clone()) {
                    frontier.push_back(obj);
                }
            }
        }

        reachable
    }

    // -- Describe -----------------------------------------------------------

    pub fn describe(&self, id: &str) -> String {
        match self.get(id) {
            None => format!("not found: {id}"),
            Some(Node::Entity(e)) => e.display_name.clone(),
            Some(Node::Statement(s)) => {
                let subj_name = self
                    .get(&s.subject_id)
                    .map(|n| n.display_name())
                    .unwrap_or(&s.subject_id);
                let obj_name = self
                    .get(&s.object_id)
                    .map(|n| n.display_name())
                    .unwrap_or(&s.object_id);
                format!("{} -[{}]-> {}", subj_name, s.predicate, obj_name)
            }
        }
    }
}
