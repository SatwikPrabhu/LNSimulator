# Lightning Network Simulator

This repository contains a Rust-based simulator for the Lightning Network (LN) payments. The simulator is designed to model the behavior of the LN payment routing without the complexities of the cryptography or communication involved.

## Modules

The project is divided into several modules:

- create_graph: This module is responsible for creating the graph structure from a JSON file that contains a snapshot of LN obtained in July 2022. The graph represents the Lightning Network.

- find_path: This module is responsible for finding the optimal path for a payment between a sender and a recipient for a given transaction amount.

- payment_router: This module is responsible for routing payments allowing concurrency and balance updates.

- payment_scheduler: This module is responsible for scheduling payments.

## Usage

To run the simulator, just run the main function in main.rs. The simulation time can be adjusted as needed using the simulation_duration variable. This will start the simulator and log its progress to a file named sim.log. The payment amounts and the delays between starting two payments can be adjusted in payment_scheduler.rs. 

## Concurrency

The simulator uses Rust's Arc and Mutex types to share the graph structure across concurrent payments. This allows the simulator to model the concurrent nature of payments in LN.

## Future Work

The simulator can be made more realistic by incorporating:
- Real world latency data to simulate latencies
- Logic to compute channel failure probabilities in the path computation
- Rerouting logic failed payments
