use std::fs::File;
use std::io::{self, BufRead};
use std::collections::{HashMap};
use std::cmp::Ordering;
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Clone,Serialize, Deserialize)]
pub struct Node {
    pub osm_id: String,
    pub latitude: f64,
    pub longitude: f64,
    tags: std::collections::HashMap<String, String>, // Using HashMap for tags
    pub adjacency_list: Vec<String>, // List of adjacency nodes
}

pub fn get_adjacency_nodes(node_id: &str) -> io::Result<Vec<Node>> {
    let filename = "output_nodes.txt";
    get_adjacency_nodes_from_file(filename, node_id)
}

fn get_adjacency_nodes_from_file(filename: &str, node_id: &str) -> io::Result<Vec<Node>> {

    // Open the file
    let file = File::open(filename)?;
    let reader = io::BufReader::new(file);
    let mut adjacency_nodes = Vec::new();

    // Iterate through each line in the file
    for line in reader.lines() {
        let mut current_osm_id = String::new();
        let mut current_adjacency_list: Vec<String> = Vec::new();

        let line = line?;

        let valid_json = line.replace("'", "\""); // Replace single quotes with double quotes

        // Parse the JSON into the struct
        let parsed_node: Value= serde_json::from_str(&valid_json)?;
        // let parsed_node: Node = serde_json::from_str(valid_json)?;

        // Print the parsed data
        println!("Parsed Data: {:?}", parsed_node);
        // // Clean the line and remove curly braces
        // let line = line.trim().trim_start_matches('{').trim_end_matches('}');

        // // Split the line into key-value pairs
        // let parts: Vec<&str> = line.split(", ").collect();

        // for part in parts {
        //     if part.contains("osm_id") {
        //         // Extract osm_id
        //         current_osm_id = part.split(':').nth(1)
        //             .unwrap_or("")
        //             .trim()
        //             .trim_matches('\'')
        //             .to_string();
        //     } else if part.contains("adjacency_list") {
        //         println!("{}", part);
        //         // Extract the adjacency list
        //         let adjacency_str = part.split(':').nth(1)
        //             .unwrap_or("")
        //             .trim();
        //         current_adjacency_list = adjacency_str
        //             .trim_start_matches('[')
        //             .trim_end_matches(']')
        //             .split(',')
        //             .map(|s| s.trim().trim_matches('\'').to_string())
        //             .collect();
        //     }
        // }
        //  // After processing the line, check if we need to save it
        //  if current_osm_id == node_id {
        //     let mut adjacency_nodes = Vec::new();
        //     for adjacent_id in current_adjacency_list {
        //         adjacency_nodes.push(Node { id: adjacent_id.clone()});
        //         println!("{}",adjacent_id);

        //     }
        //     return Ok(adjacency_nodes)
        // }


    }


    // Return the vector of edges
    Ok(adjacency_nodes)
}

// fn dijkstra(start: i32, end: i32) -> Vec<i32> {
//     let mut distances: HashMap<i32, i32> = HashMap::new();
//     let mut predecessors: HashMap<i32, i32> = HashMap::new();
//     let mut heap = BinaryHeap::new();

//     distances.insert(start, 0);
//     heap.push(State {
//         cost: 0,
//         node: start,
//     });

//     while let Some(State { cost, node }) = heap.pop() {
//         if node == end {
//             break;
//         }

//         if cost > *distances.get(&node).unwrap_or(&i32::MAX) {
//             continue;
//         }

//         match get_adjacency_nodes(node) {
//             Ok(edges) => {
//                 for edge in edges {
//                     let next = edge.to;
//                     let next_cost = cost + edge.weight;

//                     if next_cost < *distances.get(&next).unwrap_or(&i32::MAX) {
//                         distances.insert(next, next_cost);
//                         predecessors.insert(next, node);
//                         heap.push(State {
//                             cost: next_cost,
//                             node: next,
//                         });
//                     }
//                 }
//             }
//             Err(e) => eprintln!("Error reading adjacency nodes: {}", e),
//         }
//     }

//     let path = if distances.contains_key(&end) {
//         let distance = distances[&end];
//         let path = reconstruct_path(&predecessors, start, end);
//         // Print the distance and path
//         println!(
//             "Distance from node {} to node {}: {}\nPath: {:?}",
//             start, end, distance, path
//         );
//         path
//     } else {
//         // Print the error message
//         println!("No path found from node {} to node {}", start, end);
//         Vec::new()
//     };

//     path

// }


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

fn main() {
    let node_id = "103981998";

    // Call the function and get the result
    let nodes = get_adjacency_nodes(node_id);

    // println!("Adjacency Nodes for {}:", node_id);
    // for node in nodes.iter() {
    //     println!("Node ID: {}", node.id);
    // }
}
