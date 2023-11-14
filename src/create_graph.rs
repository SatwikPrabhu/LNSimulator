use petgraph::graph::{NodeIndex, DiGraph};
use serde_json::Value;
use std::fs;
use std::path::Path;

// Structure to hold the properties of every channel in the graph.
#[derive(Debug, Clone, PartialOrd, PartialEq, Copy)]
pub struct EdgeAttributes {
    pub basefee: f32,
    pub feerate: f32,
    pub balance: f32,
    pub delay: f32,
    pub age: i64,
}

// Function to convert an existing snapshot (originally in networkx format) in the networkx format to a petgraph format
pub fn convert_networkx_to_petgraph<P: AsRef<Path>>(file_path: P) -> DiGraph<usize, EdgeAttributes> {
    // Read and parse the JSON file
    let file_content = fs::read_to_string(file_path).expect("Error reading file");
    let json: Value = serde_json::from_str(&file_content).expect("Error parsing JSON");

    // Create a new Petgraph graph (Directed Graph)
    let mut graph = DiGraph::new();

    // Transfer nodes
    if let Some(nodes) = json.get("nodes") {
        for node in nodes.as_array().unwrap() {
            graph.add_node(node["id"].as_i64().unwrap() as usize);
        }
    }

    // Transfer edges and edge properties
    if let Some(edges) = json.get("links") {
        for edge in edges.as_array().unwrap() {
            let start = NodeIndex::new(edge["source"].as_u64().unwrap() as usize);
            let end = NodeIndex::new(edge["target"].as_u64().unwrap() as usize);
            let attrs = EdgeAttributes {
                basefee: edge["basefee"].as_f64().unwrap() as f32,
                feerate: edge["feerate"].as_f64().unwrap() as f32,
                delay: edge["delay"].as_i64().unwrap() as f32,
                balance: edge["balance"].as_f64().unwrap() as f32,
                age: edge["age"].as_i64().unwrap() as i64,
            };
            graph.add_edge(start, end, attrs);
        }
    }
    graph
}
