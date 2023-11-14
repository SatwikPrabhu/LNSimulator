mod create_graph;
mod find_path;
mod payment_router;
mod payment_scheduler;

use std::sync::{Arc, Mutex};
use tokio;
use tokio::time::{self, Duration};
use simplelog::*;
use std::fs::File;
use log::{info, error};

#[tokio::main]
async fn main() {
    // Initialize the logger
    WriteLogger::init(LevelFilter::Info, Config::default(), File::create("sim.log").unwrap()).unwrap();
    info!("Starting the program");
    // Obtain the graph structure from the json file.
    let graph = create_graph::convert_networkx_to_petgraph("/Users/redhawk/RustroverProjects/LNsimulator/graph_data/json_graph1.json");
    // Convert the graph so that it can be shared across concurrent payments.
    let graph_arc = Arc::new(Mutex::new(graph));
    let graph_clone = Arc::clone(&graph_arc);
    // Required simulation duration.
    let simulation_duration = Duration::from_secs(20);
    // Use `timeout` to limit the scheduler's execution time to the simulation duration.
    let result = time::timeout(simulation_duration, async {
        payment_scheduler::schedule_payments(graph_clone).await;
    }).await;

    match result {
        Ok(_) => info!("Scheduler completed successfully"),
        Err(_) => info!("Scheduler timed out"),
    }
}
