use rand::Rng; // For generating random numbers
use tokio::time::{sleep, Duration};
use petgraph::graph::{NodeIndex,DiGraph};
use std::sync::{Arc, Mutex};
use log::info;
use crate::create_graph;
use crate::find_path;
use crate::payment_router;
use create_graph::EdgeAttributes;

// Scheduler function to schedule payments asynchronously so that concurrent payments are possible.
pub async fn schedule_payments(graph: Arc<Mutex<DiGraph<usize, EdgeAttributes>>>) -> Result<(), String>{
    let mut rng = rand::thread_rng();
    let mut payment_id:u64 = 0;
    loop {
        // Generate random sender and recipient
        payment_id += 1;
        let guard = graph.lock().map_err(|e| e.to_string())?;
        let sender = NodeIndex::new(rng.gen_range(0..guard.node_count()));
        let recipient = NodeIndex::new(rng.gen_range(0..guard.node_count()));
        drop(guard);
        // Ensure sender and recipient are not the same
        if sender != recipient {
            // Use the path finding algorithm to get path, timelocks, and amounts. The payment
            // amount is assumed to be 10000 satoshis here. This can be set according to simulation
            // needs. Higher amounts can lead to more payment failures both due to balance
            // availability and no paths found.
            let (path,timelocks,amounts) = find_path::dijkstra(Arc::clone(&graph), sender, recipient,1000.0);

            // Create a Payment instance
            let mut payment = payment_router::Payment::new(payment_id,path,timelocks,amounts);

            // Schedule the payment with a random delay
            let delay = rng.gen_range(0..10); // Random delay in milliseconds. This can
            // be adjusted according to simulation needs. Smaller delays will mean more concurrent
            // payments.
            sleep(Duration::from_millis(delay)).await;
            let graph_clone = Arc::clone(&graph);
            info!("Payment no. {:?} started from {:?} to {:?}",payment_id,sender,recipient);
            // Process the payment asynchronously.
            tokio::spawn(async move {
                payment_router::Payment::payment_manager(graph_clone, &mut payment).await;
            });

        }
    }
}
