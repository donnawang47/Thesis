// In src/dijkstra.rs
use crate::database;  // Import the database module from the root
use crate::database::Node;
use aws_sdk_dynamodb as dynamodb;
use dynamodb::{types::AttributeValue, Client};

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;
use geoutils::{Location, Distance};

#[derive(Debug)]
struct State {
    cost: f64,
    node: Node,
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

fn reconstruct_path(predecessors: &HashMap<String, String>, start: &str, end: &str) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = end.to_string();

    while current != start {
        path.push(current.clone());
        if let Some(pred) = predecessors.get(&current) {
            current = pred.clone();
        } else {
            return Vec::new(); // Path not found
        }
    }

    path.push(start.to_string());
    path.reverse();
    path
}

fn get_distance(node_a: &Node, node_b: &Node) -> f64 {
    let node_a_location = Location::new(node_a.latitude, node_a.longitude);
    let node_b_location = Location::new(node_b.latitude, node_b.longitude);

    let distance = node_a_location.distance_to(&node_b_location).unwrap().meters();

    distance
}

pub async fn dijkstra(client: &Client, table_name: String,src_node: Node, dest_node: Node) -> Vec<String> {
    let mut distances: HashMap<String, f64> = HashMap::new();
    let mut predecessors: HashMap<String, String> = HashMap::new();
    let mut heap = BinaryHeap::new();


    distances.insert(src_node.id.to_string(), 0.0);
    heap.push(State {
        cost: 0.0,
        node: src_node.clone(),
    });

    while let Some(State { cost, node }) = heap.pop() {
        if node.id.to_string() == dest_node.id.to_string() {
            break;
        }

        if cost > *distances.get(&node.id.to_string()).unwrap_or(&f64::MAX) {
            continue;
        }

        for next_node_id in node.adjacency_list.clone() {
            let next_node = database::query_node_by_id(&client, table_name.to_string(), next_node_id.to_string()).await.unwrap();

            let weight = get_distance(&node, &next_node);
            let next_cost = cost + weight;

            if next_cost < *distances.get(&next_node.id).unwrap_or(&f64::MAX) {
                distances.insert(next_node.id.to_string(), next_cost);
                predecessors.insert(next_node.id.to_string(), node.id.to_string().to_string());
                heap.push(State {
                    cost: next_cost,
                    node: next_node,
                });
            }
        }
    }

    let path = if distances.contains_key(&dest_node.id.to_string()) {
        let distance = distances[&dest_node.id.to_string()];
        let path = reconstruct_path(&predecessors, &src_node.id.to_string(), &dest_node.id.to_string());
        // Print the distance and path
        println!(
            "Distance from node {} to node {}: {}\nPath: {:?}",
            src_node.id.to_string(), dest_node.id.to_string(), distance, path
        );
        path
    } else {
        // Print the error message
        println!("No path found from node {} to node {}", src_node.id.to_string(), dest_node.id.to_string());
        Vec::new()
    };

    path

}
