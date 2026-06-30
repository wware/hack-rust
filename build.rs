use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn escape_rust_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c    => out.push(c),
        }
    }
    out
}

fn read_jsonl(path: &Path) -> Vec<serde_json::Value> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("build: cannot read {}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l)
            .unwrap_or_else(|e| panic!("build: parse error in {}: {e}", path.display())))
        .collect()
}

fn truth_variant(s: &str) -> &'static str {
    match s {
        "asserted_true"  => "TruthStatus::AssertedTrue",
        "asserted_false" => "TruthStatus::AssertedFalse",
        "hypothetical"   => "TruthStatus::Hypothetical",
        "disputed"       => "TruthStatus::Disputed",
        "retracted"      => "TruthStatus::Retracted",
        other => panic!("build: unknown truth_status: {other}"),
    }
}

fn main() {
    // Link libguile when cross-compiling for x86_64 macOS (Guile demo).
    let target = env::var("TARGET").unwrap_or_default();
    if target == "x86_64-apple-darwin" {
        println!("cargo:rustc-link-search=native=/usr/local/lib");
        println!("cargo:rustc-link-lib=dylib=guile-3.0");
        println!("cargo:rustc-link-search=native=/usr/local/opt/bdw-gc/lib");
        println!("cargo:rustc-link-lib=dylib=gc");
    }

    // --- Codegen: embed JSONL data as static Rust arrays ---
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest);
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("bohemia_data.rs");

    for name in &[
        "bohemia_entities.jsonl",
        "bohemia_events.jsonl",
        "bohemia_moments.jsonl",
        "bohemia_triplets.jsonl",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(name).display());
    }

    let mut out = File::create(&out_path)
        .unwrap_or_else(|e| panic!("build: cannot create {}: {e}", out_path.display()));

    // ---- ENTITY_RECORDS ----
    let entities = read_jsonl(&root.join("bohemia_entities.jsonl"));
    writeln!(out, "pub static ENTITY_RECORDS: &[StaticEntityRecord] = &[").unwrap();
    for v in &entities {
        let entity_id = escape_rust_str(v["entity_id"].as_str().unwrap());
        let canonical = escape_rust_str(v["canonical"].as_str().unwrap());
        let kind      = escape_rust_str(v["type"].as_str().unwrap());
        let wiki_url  = v["wiki_url"].as_str().map(escape_rust_str);
        let aliases: Vec<String> = v["aliases"].as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str().map(escape_rust_str)).collect())
            .unwrap_or_default();

        writeln!(out, "    StaticEntityRecord {{").unwrap();
        writeln!(out, "        entity_id: \"{entity_id}\",").unwrap();
        writeln!(out, "        canonical: \"{canonical}\",").unwrap();
        write!(out,   "        aliases: &[").unwrap();
        for a in &aliases { write!(out, "\"{a}\",").unwrap(); }
        writeln!(out, "],").unwrap();
        writeln!(out, "        kind: \"{kind}\",").unwrap();
        match &wiki_url {
            Some(u) => writeln!(out, "        wiki_url: Some(\"{u}\"),").unwrap(),
            None    => writeln!(out, "        wiki_url: None,").unwrap(),
        }
        writeln!(out, "    }},").unwrap();
    }
    writeln!(out, "];\n").unwrap();

    // ---- EVENT_RECORDS ----
    let events = read_jsonl(&root.join("bohemia_events.jsonl"));
    writeln!(out, "pub static EVENT_RECORDS: &[StaticEventRecord] = &[").unwrap();
    for v in &events {
        let id          = escape_rust_str(v["id"].as_str().unwrap());
        let description = escape_rust_str(v["description"].as_str().unwrap());
        writeln!(out, "    StaticEventRecord {{ id: \"{id}\", description: \"{description}\" }},").unwrap();
    }
    writeln!(out, "];\n").unwrap();

    // ---- MOMENT_RECORDS ----
    let moments = read_jsonl(&root.join("bohemia_moments.jsonl"));
    writeln!(out, "pub static MOMENT_RECORDS: &[StaticMomentRecord] = &[").unwrap();
    for v in &moments {
        let id    = escape_rust_str(v["id"].as_str().unwrap());
        let label = escape_rust_str(v["label"].as_str().unwrap());
        writeln!(out, "    StaticMomentRecord {{ id: \"{id}\", label: \"{label}\" }},").unwrap();
    }
    writeln!(out, "];\n").unwrap();

    // ---- TRIPLET_RECORDS ----
    let triplets = read_jsonl(&root.join("bohemia_triplets.jsonl"));
    writeln!(out, "pub static TRIPLET_RECORDS: &[StaticTripletRecord] = &[").unwrap();
    for v in &triplets {
        let id         = escape_rust_str(v["id"].as_str().unwrap());
        let predicate  = escape_rust_str(v["predicate"].as_str().unwrap());
        let subject_id = escape_rust_str(v["subject_id"].as_str().unwrap());
        let object_id  = escape_rust_str(v["object_id"].as_str().unwrap());
        let truth      = truth_variant(v["truth_status"].as_str().unwrap());
        let story_id   = escape_rust_str(v["story_id"].as_str().unwrap());
        let para       = v["paragraph_index"].as_u64().unwrap();
        let sent_ids: Vec<u64> = v["sentence_ids"].as_array()
            .map(|a| a.iter().filter_map(|x| x.as_u64()).collect())
            .unwrap_or_default();
        let narrator   = v["asserting_narrator_id"].as_str().map(escape_rust_str);
        let confidence = v["extraction_confidence"].as_f64().unwrap();

        writeln!(out, "    StaticTripletRecord {{").unwrap();
        writeln!(out, "        id: \"{id}\",").unwrap();
        writeln!(out, "        predicate: \"{predicate}\",").unwrap();
        writeln!(out, "        subject_id: \"{subject_id}\",").unwrap();
        writeln!(out, "        object_id: \"{object_id}\",").unwrap();
        writeln!(out, "        truth_status: {truth},").unwrap();
        writeln!(out, "        story_id: \"{story_id}\",").unwrap();
        writeln!(out, "        paragraph_index: {para}_u32,").unwrap();
        write!(out,   "        sentence_ids: &[").unwrap();
        for sid in &sent_ids { write!(out, "{sid}_u32,").unwrap(); }
        writeln!(out, "],").unwrap();
        match &narrator {
            Some(n) => writeln!(out, "        asserting_narrator_id: Some(\"{n}\"),").unwrap(),
            None    => writeln!(out, "        asserting_narrator_id: None,").unwrap(),
        }
        writeln!(out, "        extraction_confidence: {confidence:?}_f64,").unwrap();
        writeln!(out, "    }},").unwrap();
    }
    writeln!(out, "];").unwrap();
}
