use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::types::{
    EntityNode, EntityRecord, EventRecord, MomentRecord, Node, StatementNode, TripletRecord,
};

// ---------------------------------------------------------------------------
// JSONL helpers
// ---------------------------------------------------------------------------

fn read_jsonl<T, P>(path: P) -> Result<Vec<T>, String>
where
    T: serde::de::DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let file = File::open(path).map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for (line_no, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("{}: line {line_no}: {e}", path.display()))?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: T = serde_json::from_str(line)
            .map_err(|e| format!("{}: line {line_no}: {e}\n  content: {line}", path.display()))?;
        records.push(record);
    }
    Ok(records)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub struct LoadedData {
    pub nodes: Vec<Node>,
    pub warnings: Vec<String>,
}

pub fn load_graph_data(
    entities_path: &Path,
    events_path: &Path,
    moments_path: &Path,
    triplets_path: &Path,
    sentence_cutoff: Option<u32>,
) -> Result<LoadedData, String> {
    let mut nodes: Vec<Node> = Vec::new();
    let warnings: Vec<String> = Vec::new();

    // -- Entities -----------------------------------------------------------
    let entity_records: Vec<EntityRecord> = read_jsonl(entities_path)?;
    for rec in entity_records {
        nodes.push(Node::Entity(EntityNode {
            id: rec.entity_id,
            display_name: rec.canonical,
            aliases: rec.aliases,
            kind: rec.kind,
            wiki_url: rec.wiki_url,
        }));
    }

    // -- Events -------------------------------------------------------------
    let event_records: Vec<EventRecord> = read_jsonl(events_path)?;
    for rec in event_records {
        nodes.push(Node::Entity(EntityNode {
            id: rec.id,
            display_name: rec.description,
            aliases: vec![],
            kind: "event".to_string(),
            wiki_url: None,
        }));
    }

    // -- Moments ------------------------------------------------------------
    let moment_records: Vec<MomentRecord> = read_jsonl(moments_path)?;
    for rec in moment_records {
        nodes.push(Node::Entity(EntityNode {
            id: rec.id,
            display_name: rec.label,
            aliases: vec![],
            kind: "moment".to_string(),
            wiki_url: None,
        }));
    }

    // -- Triplets -----------------------------------------------------------
    let triplet_records: Vec<TripletRecord> = read_jsonl(triplets_path)?;
    for rec in triplet_records {
        // Apply sentence_cutoff: skip triplets where any sentence_id >= cutoff
        if let Some(cutoff) = sentence_cutoff {
            if rec.sentence_ids.iter().any(|&s| s >= cutoff) {
                continue;
            }
        }

        nodes.push(Node::Statement(StatementNode {
            id: rec.id,
            predicate: rec.predicate,
            subject_id: rec.subject_id,
            object_id: rec.object_id,
            truth_status: rec.truth_status,
            story_id: rec.story_id,
            paragraph_index: rec.paragraph_index,
            sentence_ids: rec.sentence_ids,
            asserting_narrator_id: rec.asserting_narrator_id,
            extraction_confidence: rec.extraction_confidence,
        }));
    }

    if warnings.is_empty() {
        println!(
            "Loaded {} nodes ({} entities/events/moments, {} statements)",
            nodes.len(),
            nodes.iter().filter(|n| n.is_entity()).count(),
            nodes.iter().filter(|n| n.is_statement()).count(),
        );
    } else {
        eprintln!("{} warnings during load", warnings.len());
        for w in &warnings {
            eprintln!("  [warn] {w}");
        }
    }

    Ok(LoadedData { nodes, warnings })
}
