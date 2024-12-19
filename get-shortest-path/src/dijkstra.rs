// In src/dijkstra.rs
use crate::database;  // Import the database module from the root
use crate::database::RawNode;

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

#[derive(Debug)]
struct State {
    cost: f64,
    node: RawNode,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // This will create a max-heap based on cost
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Handle NaN cases, though cmp handles it correctly
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

    // Convert node IDs to coordinates as 2D array [longitude, latitude]
    path.iter().map(|node_id| {
        let node = nodes.get(node_id).unwrap();
        [node.lon, node.lat]  // Return as 2D array [longitude, latitude]
    }).collect()
}

fn get_distance(node_a: &RawNode, node_b: &RawNode) -> f64 {
    // Calculate the distance between two nodes based on their coordinates
    let lon_a = node_a.lon;
    let lat_a = node_a.lat;
    let lon_b = node_b.lon;
    let lat_b = node_b.lat;

    // Use geoutils or another method for calculating the actual distance between coordinates
    let node_a_location = geoutils::Location::new(lat_a, lon_a);
    let node_b_location = geoutils::Location::new(lat_b, lon_b);

    let distance = node_a_location.distance_to(&node_b_location).unwrap().meters();
    distance
}

pub async fn dijkstra(pool: &sqlx::PgPool, src_node: RawNode, dest_node: RawNode) -> Vec<[f64; 2]> {
    // key = id, val = distance
    let mut distances: HashMap<i64, f64> = HashMap::new();
    // key = id, val = id
    let mut predecessors: HashMap<i64, i64> = HashMap::new();
    let mut nodes: HashMap<i64, RawNode> = HashMap::new(); // To store nodes
    let mut heap = BinaryHeap::new();

    distances.insert(src_node.id, 0.0);
    heap.push(State {
        cost: 0.0,
        node: src_node.clone(),
    });
    nodes.insert(src_node.id, src_node.clone()); // Add the source node to the nodes map

    while let Some(State { cost, node }) = heap.pop() {
        if node.id.to_string() == dest_node.id.to_string() {
            break;
        }

        if cost > *distances.get(&node.id).unwrap_or(&f64::MAX) {
            continue;
        }

        for next_node_id in node.adjacency_list.clone() {
            let next_node = database::get_node_by_id(&pool, next_node_id).await.unwrap();

            let weight = get_distance(&node, &next_node);
            let next_cost = cost + weight;

            if next_cost < *distances.get(&next_node.id).unwrap_or(&f64::MAX) {
                distances.insert(next_node.id, next_cost);
                predecessors.insert(next_node.id, node.id);
                heap.push(State {
                    cost: next_cost,
                    node: next_node.clone(),
                });
                nodes.insert(next_node.id, next_node); // Add the node to the nodes map
            }
        }
    }

    let path = if distances.contains_key(&dest_node.id) {
        let distance = distances[&dest_node.id];
        let path_coords = reconstruct_path(&predecessors, src_node.id, dest_node.id, &nodes);
        // Print the distance and path
        // println!(
        //     "Distance from node {} to node {}: {}\nPath: {:?}",
        //     src_node.id.to_string(),
        //     dest_node.id.to_string(),
        //     distance,
        //     path_coords
        // );
        path_coords
    } else {
        // Print the error message
        println!("No path found from node {} to node {}", src_node.id.to_string(), dest_node.id.to_string());
        Vec::new()
    };

    path
}
