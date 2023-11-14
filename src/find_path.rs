use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use crate::create_graph::EdgeAttributes;
use ordered_float::OrderedFloat;
use std::sync::{Arc, Mutex};
use log::info;

// Risk factor for locking funds. This basically quantifies the cost for locking unit value for unit
// time. The cost function uses this value to compute channel cost for addition to the optimal path.
// This value can be changed as per the need of the simulator.
const RF: f32 = 0.0000000015;
// A struct to represent items in the priority queue. The priority queue  is used to construct the
// best path.
#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: OrderedFloat<f32>,
    position: NodeIndex,
}

// Implement ordering for States. We want to pop the lowest cost, not the highest,
// so we reverse the comparison.
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap()
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Cost function as per LND, the most popular lightning routing client.
pub fn lnd_cost_fn(attrs:EdgeAttributes,amt:f32) -> f32 {
    let wght = amt*attrs.delay*RF + attrs.basefee + amt*attrs.feerate;
    wght
}

// The cost for the first channel is a little different as the sender does not collect fees.
pub fn lnd_cost_fn_snd(attrs:EdgeAttributes,amt:f32) -> f32 {
    let wght = amt*attrs.delay*RF;
    wght
}

// Function to compute the best path from the sender to the recipient for a certain transaction
// amount. The path computation is based on Dijkstra's algorithm but with the LND cost function.
pub fn dijkstra(graph: Arc<Mutex<DiGraph<usize, EdgeAttributes>>>, r: NodeIndex, s:NodeIndex, amt:f32) -> (Vec<NodeIndex>,Vec<f32>,Vec<f32>){

    // Lock the shared graph for computing the path.
    let guard = match graph.lock() {
        Ok(g) => g,
        Err(e) => {
            // Handle the error, e.g., by logging, returning an error, or retrying
            info!("Failed to acquire lock: {}", e);
            return (Vec::new(), Vec::new(),Vec::new());
        }
    };

    // Initialize the distance map and the values of the timelocks and amounts (including fees) for
    // every node in the graph.
    let mut dist: HashMap<NodeIndex, OrderedFloat<f32>> = guard.node_indices().map(|n| (n, OrderedFloat(f32::INFINITY))).collect();
    let mut timelock: HashMap<NodeIndex, f32> = guard.node_indices().map(|n| (n, 0.0)).collect();
    let mut amount: HashMap<NodeIndex, f32> = guard.node_indices().map(|n| (n, 0.0)).collect();
    let mut predecessors: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    let mut heap = BinaryHeap::new();

    // Fill the values for the recipient. The search starts from the recipient as the fees of a node
    // in the path is calculated based on the amount that it has to forward. Add the recipient to
    // the priority queue.
    dist.insert(r,OrderedFloat(0.0));
    timelock.insert(r, 0.0);
    amount.insert(r,amt);
    heap.push(State { cost: OrderedFloat(0.0), position: r});

    // Main loop to compute the best path based on Dijkstra's algorithm.
    while let Some(State { cost, position}) = heap.pop() {
        if cost > dist[&position] {
            continue;
        }
        // If the current node is the sender, then the best path has been found. Return the path
        // along with the respective timelocks and amounts.
        if position == s {
            let mut path = vec![s];
            let mut delays = vec![timelock[&s]];
            let mut amounts = vec![amount[&s]];
            let mut current = s;
            while let Some(&predecessor) = predecessors.get(&current) {
                path.push(predecessor);
                delays.push(timelock[&predecessor] / 1000.0);
                amounts.push(amount[&predecessor]);
                current = predecessor;
                if current == s {
                    break;
                }
            }
            return ( path, delays, amounts);
        }
        // Update best paths for every neighbor of the current best node. If the sender is a
        // neighbor, the cost is calculated in a different manner as compared to non-senders.
        // Additionally, we keep in mind that the sender knows its balances but not the balances of
        // other channels.
        for neighbor in guard.neighbors_directed(position, Direction::Incoming) {
            if let Some(edge1) = guard.find_edge(neighbor, position) {
                let attrs = &guard[edge1];
                let mut next_cost:OrderedFloat<f32> = OrderedFloat(f32::INFINITY);
                if neighbor == s{
                    next_cost = OrderedFloat(cost.into_inner() + lnd_cost_fn_snd(attrs.clone(), amount[&position]));
                }else{
                    next_cost = OrderedFloat(cost.into_inner() + lnd_cost_fn(attrs.clone(), amount[&position]));
                }
                if let Some(edge) = guard.find_edge(neighbor, position) {
                    let attrs = &guard[edge];
                    if next_cost < *dist.get(&neighbor).unwrap_or(&OrderedFloat(f32::INFINITY)) {
                        if (attrs.balance >= amount[&position] && neighbor ==s) || (attrs.balance + attrs.balance >= amount[&position] && neighbor!=s)  {
                            heap.push(State { cost: next_cost, position: neighbor });
                            dist.insert(neighbor, next_cost);
                            timelock.insert(neighbor, timelock[&position] + attrs.delay);
                            amount.insert(neighbor, amount[&position] + attrs.basefee + amount[&position] * attrs.feerate);
                            predecessors.insert(neighbor, position);
                        }
                    }
                }
            }


        }
    }

    // Drop the lock the shared lock
    drop(guard);
    (Vec::new(), Vec::new(),Vec::new())
}

