use petgraph::graph::{NodeIndex,DiGraph};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use log::{error, info};
use crate::create_graph;
use create_graph::EdgeAttributes;
use tokio::time::{sleep};

// Define the payment structure.
pub struct Payment {
    payment_id: u64, // Payment ID
    path: Vec<NodeIndex>, // Payment path
    timelocks: Vec<f32>, // Timelocks of every node in the path
    amounts: Vec<f32>, // Amounts at every node in the path
    lock_status: Vec<bool>, // Tracker of which nodes in the path have locked
    lock_time: Vec<Option<Instant>>, // Tracker of the time of locking for every node in the path
    secret_key_status: Vec<bool>, // Tracker of whether a node in the path has the secret key
    secret_key_received_time: Vec<Option<Instant>>, // Tracker of the time of receiving the secret
                                                    // key for every node in the path
    unlock_status: Vec<bool>, // Tracker of whether a node has unlocked its locked funds
    timelock_expired: Vec<bool>, // Tracker of whether the timelock of a node has expired
}

// Functions related to a payment
impl Payment {
    // Initialize a new Payment with the values returned by the path finding algorithm.
    pub fn new(payment_id: u64, path: Vec<NodeIndex>, timelocks: Vec<f32>, amounts: Vec<f32>) -> Self {
        Payment {
            payment_id,
            path: path.clone(),
            timelocks,
            amounts,
            lock_status: vec![false; path.len()],
            lock_time: vec![None; path.len()],
            secret_key_status: vec![false; path.len()],
            secret_key_received_time: vec![None; path.len()],
            unlock_status:vec![false; path.len()],
            timelock_expired: vec![false; path.len()]
        }
    }

    // Function to check if a node can lock funds.
    pub async fn lock_funds(graph: Arc<Mutex<DiGraph<usize, EdgeAttributes>>>, payment: &mut Payment, node_index: usize, amount: f32) -> Result<(), String> {
        // Obtain lock on the shared graph to do the locking process
        let guard = graph.lock().map_err(|e| e.to_string())?;
        if node_index + 1 >= payment.path.len() {
            return Err("Invalid node index".to_string());
        }
        // Check if the node has sufficient balance to lock. If yes, then lock and set the lock
        // status.
        if let Some(edge) = guard.find_edge(payment.path[node_index], payment.path[node_index + 1]){
            let mut attrs = guard[edge];

            if attrs.balance < amount {
                info!("Insufficient balance at node {:?} for payment id {:?}", payment.path[node_index],payment.payment_id);
            }else{
                attrs.balance -= amount;
                payment.lock_status[node_index] = true;
                payment.lock_time[node_index] = Some(Instant::now());
                info!("Locked amount by node {:?} for payment id {:?}", payment.path[node_index], payment.payment_id);
                // If this is the penultimate node, then by locking it automatically notifies
                // the recipient of the payment and the recipient shares the secret key with this
                // node and the secret key status is updated accordingly.
                if node_index == payment.path.len() - 2 {
                    // Set secret key received for the last node
                    payment.secret_key_status[payment.path.len() - 1] = true;
                    payment.secret_key_received_time[payment.path.len() - 1] = Some(Instant::now());
                    payment.secret_key_status[payment.path.len() - 2] = true;
                    payment.secret_key_received_time[payment.path.len() - 2] = Some(Instant::now());
                    info!("Secret shared by recipient {:?} for payment id {:?}", payment.path[payment.path.len() - 1], payment.payment_id);
                    info!("Secret received by node {:?} for payment id {:?}", payment.path[payment.path.len() - 2], payment.payment_id);
                }
            }


        }
        // Drop the lock on the shared graph
        drop(guard);


        Ok(())
    }

    // Function to lock the funds in the first channel. The sender inherently has the sufficient
    // balance due to the nature of the path finding algorithm. Accordingly update the lock status
    // of the sender.
    pub async fn lock_funds_sender(graph: Arc<Mutex<DiGraph<usize, EdgeAttributes>>>, payment: &mut Payment) -> Result<(), String> {
        let guard = graph.lock().map_err(|e| e.to_string())?;
        if let Some(edge) = guard.find_edge(payment.path[0], payment.path[1]){
            let mut attrs = guard[edge];
            attrs.balance -= payment.amounts[0];
            payment.lock_status[0] = true;
            payment.lock_time[0] = Some(Instant::now());
            info!("Locked amount by sender {:?} for payment id {:?}", payment.path[0], payment.payment_id);

        }
        drop(guard);
        Ok(())
    }

    // Function to check whether a node has been updated with the secret key. If yes, then it
    // unlocks the locked funds with the next node and sets the secret key status of the
    // predecessor. Otherwise, it checks whether the node's timelock has expired by checking the
    // time elapsed since the node locked. If the timelock has expired it updates the relevant field
    // in the payment structure and reverts the locked funds.
    pub async fn check_secret_key(graph: Arc<Mutex<DiGraph<usize, EdgeAttributes>>>, payment: &mut Payment, node_index: usize) -> Result<(), String> {

        if node_index >= payment.path.len() {
            return Err("Invalid node index".to_string());
        }
        let amount = payment.amounts[node_index];

        if payment.secret_key_status[node_index] {
            // Secret key has been received for the current node
            if node_index > 0 {
                // Set secret key for the previous node
                Payment::set_secret_key(payment, node_index).await?;
            }
            let guard = graph.lock().map_err(|e| e.to_string())?;
            // The unlocked funds have to be added to the balance in the opposite direction of the
            // channel.
            if let Some(edge) = guard.find_edge(payment.path[node_index+1], payment.path[node_index]) {
                let mut attrs = guard[edge];
                attrs.balance += amount;
                payment.unlock_status[node_index] = true;
                info!("Payment unlocked by node {:?} for payment id {:?}", payment.path[node_index], payment.payment_id);
            }
            drop(guard);
        } else {
            let lock_time = payment.lock_time[node_index].ok_or("Lock time not set")?;
            // Check if the timelock is expired. The simulated time can be adjustedaccording to
            // simulation needs. Here, the timelock value is divided by 100.
            if lock_time.elapsed().as_secs() as f32 > payment.timelocks[node_index]/100.0 {
                let guard = graph.lock().map_err(|e| e.to_string())?;
                if let Some(edge) = guard.find_edge(payment.path[node_index], payment.path[node_index+1]) {
                    let mut attrs = guard[edge];
                    attrs.balance += amount;
                    error!("Timelock reached for node {:?} for payment id {:?}", payment.path[node_index], payment.payment_id);
                }
                drop(guard);
                payment.timelock_expired[node_index] = true; // Set timelock_expired to true
            }
        }

        Ok(())
    }

    // Function to imitate sharing of the secret key with the predecessor and logging the time of
    // sharing.
    pub async fn set_secret_key(payment: &mut Payment, node_index: usize) -> Result<(), String> {
        if node_index == 0 || node_index >= payment.path.len() {
            return Err("Invalid node index".to_string());
        }

        payment.secret_key_status[node_index - 1] = true;
        payment.secret_key_received_time[node_index - 1] = Some(Instant::now());
        info!("Secret received by node {:?} for payment {:?}", payment.path[node_index-1],payment.payment_id );

        Ok(())
    }

    // Function to manage the payment.
    pub async fn payment_manager(graph: Arc<Mutex<DiGraph<usize, EdgeAttributes>>>, payment: &mut Payment) -> Result<(), String> {
        info!("Payment path {:?}, amounts {:?}, delays {:?} for payment id {:?}",payment.path, payment.timelocks, payment.amounts, payment.payment_id);
        // First check if the path is valid. Otherwise fail the payment immediately.
        if payment.path.len()<2{
            error!("Payment {:?} failed due to no path!", payment.payment_id);
            return Err("Payment failed due to no path found".to_string());
        }else{
            Payment::lock_funds_sender(Arc::clone(&graph), payment).await?;
        }

        // If the payment path is valid have a loop to continuously check the values in the payment
        // structure and call relevant functions.
        loop {
            // 1) Check if payment has succeeded or failed
            if payment.secret_key_status.iter().all(|&status| status) {
                // All secret keys received, payment succeeded
                info!("Payment {:?} success!", payment.payment_id);
                return Ok(());
            } else if payment.lock_status.iter().zip(payment.timelock_expired.iter()).all(|(&lock, &expired)| !lock || expired) {
                // All locked nodes have timelocks expired, payment failed
                error!("Payment {:?} failed due to timelock expiry!", payment.payment_id);
                return Err("Payment failed due to expired timelocks".to_string());
            }

            // 2) Iterate over nodes in the path
            for i in 0..payment.path.len()-1 {
                if i > 0 && payment.lock_status[i - 1] && !payment.lock_status[i] {
                    // Previous node locked, current node not locked
                    Payment::lock_funds(Arc::clone(&graph), payment, i, payment.amounts[i]).await?;
                } else if payment.lock_status[i] && !payment.unlock_status[i] &&!payment.timelock_expired[i] {
                    // Current node is already locked, check secret key
                    Payment::check_secret_key(Arc::clone(&graph), payment, i).await?;
                }
            }

            // Sleep for 1ms before repeating the loop
            sleep(Duration::from_millis(1)).await;
        }
    }
}
