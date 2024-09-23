// In src/dijkstra.rs
use crate::database;  // Import the database module from the root

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

#[derive(Debug)]
struct State {
    cost: i32,
    node: i32,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

impl Eq for State {}

fn reconstruct_path(predecessors: &HashMap<i32, i32>, start: i32, end: i32) -> Vec<i32> {
    let mut path = Vec::new();
    let mut current = end;

    while current != start {
        path.push(current);
        if let Some(&pred) = predecessors.get(&current) {
            current = pred;
        } else {
            return Vec::new(); // Path not found
        }
    }

    path.push(start);
    path.reverse();
    path
}


pub fn dijkstra(start: i32, end: i32) -> Vec<i32> {
    let mut distances: HashMap<i32, i32> = HashMap::new();
    let mut predecessors: HashMap<i32, i32> = HashMap::new();
    let mut heap = BinaryHeap::new();

    distances.insert(start, 0);
    heap.push(State {
        cost: 0,
        node: start,
    });

    while let Some(State { cost, node }) = heap.pop() {
        if node == end {
            break;
        }

        if cost > *distances.get(&node).unwrap_or(&i32::MAX) {
            continue;
        }

        match database::get_adjacency_nodes(node) {

            Ok(edges) => {
                for edge in edges {
                    let next = edge.to;
                    let next_cost = cost + edge.weight;

                    if next_cost < *distances.get(&next).unwrap_or(&i32::MAX) {
                        distances.insert(next, next_cost);
                        predecessors.insert(next, node);
                        heap.push(State {
                            cost: next_cost,
                            node: next,
                        });
                    }
                }
            }
            Err(e) => eprintln!("Error reading adjacency nodes: {}", e),
        }
    }

    let path = if distances.contains_key(&end) {
        let distance = distances[&end];
        let path = reconstruct_path(&predecessors, start, end);
        // Print the distance and path
        println!(
            "Distance from node {} to node {}: {}\nPath: {:?}",
            start, end, distance, path
        );
        path
    } else {
        // Print the error message
        println!("No path found from node {} to node {}", start, end);
        Vec::new()
    };

    path

}
