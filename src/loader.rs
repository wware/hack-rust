use crate::data::{ENTITY_RECORDS, EVENT_RECORDS, MOMENT_RECORDS, TRIPLET_RECORDS};
use crate::types::{EntityNode, Node, StatementNode};

pub struct LoadedData {
    pub nodes:    Vec<Node>,
    pub warnings: Vec<String>,
}

pub fn load_graph_data(sentence_cutoff: Option<u32>) -> LoadedData {
    let mut nodes: Vec<Node> = Vec::new();

    for rec in ENTITY_RECORDS {
        nodes.push(Node::Entity(EntityNode {
            id:           rec.entity_id.to_string(),
            display_name: rec.canonical.to_string(),
            aliases:      rec.aliases.iter().map(|s| s.to_string()).collect(),
            kind:         rec.kind.to_string(),
            wiki_url:     rec.wiki_url.map(|s| s.to_string()),
        }));
    }

    for rec in EVENT_RECORDS {
        nodes.push(Node::Entity(EntityNode {
            id:           rec.id.to_string(),
            display_name: rec.description.to_string(),
            aliases:      vec![],
            kind:         "event".to_string(),
            wiki_url:     None,
        }));
    }

    for rec in MOMENT_RECORDS {
        nodes.push(Node::Entity(EntityNode {
            id:           rec.id.to_string(),
            display_name: rec.label.to_string(),
            aliases:      vec![],
            kind:         "moment".to_string(),
            wiki_url:     None,
        }));
    }

    for rec in TRIPLET_RECORDS {
        if let Some(cutoff) = sentence_cutoff {
            if rec.sentence_ids.iter().any(|&s| s >= cutoff) {
                continue;
            }
        }
        nodes.push(Node::Statement(StatementNode {
            id:                    rec.id.to_string(),
            predicate:             rec.predicate.to_string(),
            subject_id:            rec.subject_id.to_string(),
            object_id:             rec.object_id.to_string(),
            truth_status:          rec.truth_status.clone(),
            story_id:              rec.story_id.to_string(),
            paragraph_index:       rec.paragraph_index,
            sentence_ids:          rec.sentence_ids.to_vec(),
            asserting_narrator_id: rec.asserting_narrator_id.map(|s| s.to_string()),
            extraction_confidence: rec.extraction_confidence,
        }));
    }

    println!(
        "Loaded {} nodes ({} entities/events/moments, {} statements)",
        nodes.len(),
        nodes.iter().filter(|n| n.is_entity()).count(),
        nodes.iter().filter(|n| n.is_statement()).count(),
    );

    LoadedData { nodes, warnings: vec![] }
}
