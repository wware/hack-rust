use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Truth status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TruthStatus {
    AssertedTrue,
    AssertedFalse,
    Hypothetical,
    Disputed,
    Retracted,
}

impl TruthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TruthStatus::AssertedTrue => "asserted_true",
            TruthStatus::AssertedFalse => "asserted_false",
            TruthStatus::Hypothetical => "hypothetical",
            TruthStatus::Disputed => "disputed",
            TruthStatus::Retracted => "retracted",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "asserted_true" => Some(TruthStatus::AssertedTrue),
            "asserted_false" => Some(TruthStatus::AssertedFalse),
            "hypothetical" => Some(TruthStatus::Hypothetical),
            "disputed" => Some(TruthStatus::Disputed),
            "retracted" => Some(TruthStatus::Retracted),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Entity types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Person,
    Location,
    Object,
    Document,
    Event,
    Moment,
    Plan,
    Other,
}

// Raw JSONL record for an entity (bohemia_entities.jsonl)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRecord {
    pub entity_id: String,
    pub canonical: String,
    pub aliases: Vec<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub wiki_url: Option<String>,
}

// Raw JSONL record for an event (bohemia_events.jsonl)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: String,
    pub description: String,
    pub sentence_ids: Vec<u32>,
    pub para: u32,
    pub participants: Vec<String>,
    pub extraction_confidence: f64,
}

// Raw JSONL record for a moment (bohemia_moments.jsonl)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentRecord {
    pub id: String,
    pub label: String,
    pub event_id: Option<String>,
    pub narrator_id: Option<String>,
    pub sentence_ids: Vec<u32>,
    pub extraction_confidence: f64,
}

// Raw JSONL record for a triplet (bohemia_triplets.jsonl)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripletRecord {
    pub id: String,
    pub predicate: String,
    pub subject_id: String,
    pub subject_type: String,
    pub object_id: String,
    pub object_type: String,
    pub truth_status: TruthStatus,
    pub story_id: String,
    pub paragraph_index: u32,
    pub sentence_ids: Vec<u32>,
    pub asserting_narrator_id: Option<String>,
    pub extraction_method: String,
    pub extraction_confidence: f64,
    pub narrator_confidence: Option<f64>,
}

// Raw JSONL record for a sentence (bohemia_sentences.jsonl)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceRecord {
    pub id: u32,
    pub para: u32,
    pub text: String,
}

// ---------------------------------------------------------------------------
// Unified graph node — either an entity/event/moment or a statement (triplet)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "node_kind", rename_all = "snake_case")]
pub enum Node {
    Entity(EntityNode),
    Statement(StatementNode),
}

impl Node {
    pub fn id(&self) -> &str {
        match self {
            Node::Entity(e) => &e.id,
            Node::Statement(s) => &s.id,
        }
    }

    pub fn is_entity(&self) -> bool {
        matches!(self, Node::Entity(_))
    }

    pub fn is_statement(&self) -> bool {
        matches!(self, Node::Statement(_))
    }

    pub fn display_name(&self) -> &str {
        match self {
            Node::Entity(e) => &e.display_name,
            Node::Statement(s) => &s.predicate,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityNode {
    pub id: String,
    pub display_name: String,
    pub aliases: Vec<String>,
    pub kind: String,
    pub wiki_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementNode {
    pub id: String,
    pub predicate: String,
    pub subject_id: String,
    pub object_id: String,
    pub truth_status: TruthStatus,
    pub story_id: String,
    pub paragraph_index: u32,
    pub sentence_ids: Vec<u32>,
    pub asserting_narrator_id: Option<String>,
    pub extraction_confidence: f64,
}

// ---------------------------------------------------------------------------
// Edge filter options (mirrors Python edges_from/edges_to kwargs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct EdgeFilter {
    pub pred_type: Option<String>,
    pub truth: Option<TruthStatus>,
}
