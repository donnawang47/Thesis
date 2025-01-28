use crate::database; // Import the database module from the root
use crate::database::RawNode;

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use tokio::time::{timeout, Duration, Instant};

use lambda_runtime::tracing::{info, debug, error};

#[derive(Debug)]
struct State {
    cost: f64,
    node: RawNode,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.cost.partial_cmp(&other.cost)
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for State {}

fn reconstruct_path(predecessors: &HashMap<i64, i64>, start: i64, end: i64, nodes: &HashMap<i64, RawNode>) -> Vec<[f64; 2]> {
    let mut path = Vec::new();
    let mut current = end;

    while current != start {
        path.push(current.clone());
        if let Some(pred) = predecessors.get(&current) {
            current = pred.clone();
        } else {
            return Vec::new(); // Path not found
        }
    }

    path.push(start);
    path.reverse();

    path.iter()
        .map(|node_id| {
            let node = nodes.get(node_id).unwrap();
            [node.lon, node.lat] // Return as 2D array [longitude, latitude]
        })
        .collect()
}

fn get_distance(node_a: &RawNode, node_b: &RawNode) -> f64 {
    let node_a_location = geoutils::Location::new(node_a.lat, node_a.lon);
    let node_b_location = geoutils::Location::new(node_b.lat, node_b.lon);

    node_a_location.distance_to(&node_b_location).unwrap().meters()
}




pub async fn dijkstra(pool: &sqlx::PgPool, src_node: RawNode, dest_node: RawNode) -> Vec<[f64; 2]> {
    info!("Dijkstra execution started");

    let timeout_duration = Duration::from_secs(10); // Set Lambda timeout limit minus buffer
    let start_time = Instant::now(); // Record start time

    let mut distances: HashMap<i64, f64> = HashMap::new();
    let mut predecessors: HashMap<i64, i64> = HashMap::new();
    let mut nodes: HashMap<i64, RawNode> = HashMap::new();
    let mut heap = BinaryHeap::new();

    let mut query_count = 0; // Query counter
    let mut query_times = Vec::new(); // To store query times

    distances.insert(src_node.id, 0.0);
    heap.push(State {
        cost: 0.0,
        node: src_node.clone(),
    });
    nodes.insert(src_node.id, src_node.clone());

    let mut best_path = Vec::new(); // Variable to track the best path found so far
    let mut last_computed_node = src_node.clone(); // Track the last computed node

    let dijkstra_result = timeout(timeout_duration, async {
        while let Some(State { cost, node }) = heap.pop() {
            // Check if we have reached the destination node
            if node.id == dest_node.id {
                best_path = reconstruct_path(&predecessors, src_node.id, dest_node.id, &nodes);
                break;
            }

            if cost > *distances.get(&node.id).unwrap_or(&f64::MAX) {
                continue;
            }

            // Keep track of the last node processed before timeout
            last_computed_node = node.clone();

            // Process adjacent nodes
            for next_node_id in node.adjacency_list.clone() {
                let query_start = Instant::now(); // Start timing the query
                query_count += 1; // Increment query counter

                let next_node = match database::get_node_by_id(&pool, next_node_id).await {
                    Ok(node) => node,
                    Err(err) => {
                        error!("Error fetching node with ID {}: {:?}", next_node_id, err);
                        continue;
                    }
                };
                let query_duration = query_start.elapsed(); // Calculate query time
                query_times.push(query_duration);

                let weight = get_distance(&node, &next_node);
                let next_cost = cost + weight;

                if next_cost < *distances.get(&next_node.id).unwrap_or(&f64::MAX) {
                    distances.insert(next_node.id, next_cost);
                    predecessors.insert(next_node.id, node.id);
                    heap.push(State {
                        cost: next_cost,
                        node: next_node.clone(),
                    });
                    nodes.insert(next_node.id, next_node);
                }
            }
        }

        // If path was found, return it, else return empty Vec
        if distances.contains_key(&dest_node.id) {
            best_path = reconstruct_path(&predecessors, src_node.id, dest_node.id, &nodes);
        }

        Some(best_path)
    })
    .await;

    // Log summary statistics
    let elapsed_time = start_time.elapsed();
    info!(
        "Dijkstra execution completed in {:?}. Total queries: {}",
        elapsed_time, query_count
    );

    if !query_times.is_empty() {
        let min_query_time = query_times.iter().min().unwrap();
        let max_query_time = query_times.iter().max().unwrap();
        let avg_query_time: f64 = query_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / query_times.len() as f64;

        info!(
            "Query statistics - Min: {:?}, Max: {:?}, Avg: {:.4} seconds",
            min_query_time, max_query_time, avg_query_time
        );
    }

    match dijkstra_result {
        Ok(Some(path)) => path,
        Ok(None) => {
            info!(
                "No path found from node {} to node {}. Execution time: {:?}",
                src_node.id, dest_node.id, elapsed_time
            );
            Vec::new()
        }
        Err(_) => {
            error!(
                "Dijkstra function timed out after {:?}. Queries executed: {}",
                timeout_duration, query_count
            );
            // Return the path to the last computed node
            let path_to_last_node = reconstruct_path(&predecessors, src_node.id, last_computed_node.id, &nodes);
            path_to_last_node
        }
    }
}
