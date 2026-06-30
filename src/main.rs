use bohemia_graph::graph::Graph;
use bohemia_graph::loader::load_graph_data;
use bohemia_graph::types::EdgeFilter;

fn main() {
    let data = load_graph_data(None);

    let graph = Graph::new(data.nodes);

    println!("\n--- describe ---");
    println!("{}", graph.describe("wiki:Sherlock_Holmes"));
    println!("{}", graph.describe("wiki:Irene_Adler"));

    println!("\n--- edges from Holmes (all) ---");
    let filter = EdgeFilter { pred_type: None, truth: None };
    for stmt in graph.edges_from("wiki:Sherlock_Holmes", &filter) {
        println!("  {}", graph.describe(&stmt.id));
    }

    println!("\n--- BFS from Holmes, 2 hops ---");
    let layers = graph.bfs(&["wiki:Sherlock_Holmes"], 2, None);
    for (i, layer) in layers.iter().enumerate() {
        let mut ids: Vec<_> = layer.iter().collect();
        ids.sort();
        println!("  layer {i}: {} nodes", ids.len());
        for id in ids.iter().take(5) {
            println!("    {}", graph.describe(id));
        }
        if ids.len() > 5 {
            println!("    ... and {} more", ids.len() - 5);
        }
    }

    println!("\n--- transitive closure: LocatedIn from Baker Street ---");
    let reachable = graph.transitive_closure("place:baker_street", "LocatedIn");
    let mut ids: Vec<_> = reachable.iter().collect();
    ids.sort();
    for id in &ids {
        println!("  {}", graph.describe(id));
    }
    if ids.is_empty() {
        println!("  (none — entity may not be in the graph or no LocatedIn edges)");
    }
}
