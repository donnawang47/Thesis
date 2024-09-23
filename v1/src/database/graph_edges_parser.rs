use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
struct Vertex {
    id: i32,
}

#[derive(Debug)]
struct Edge {
    from: String,
    to: String,
}

fn get_node_id_from_coordinates(longitude: &str, latitude: &str) -> Option<String> {
    let path = "output_nodes.txt";

    // Open the file
    let file = File::open(path).expect("Could not open file");
    let reader = io::BufReader::new(file);

    // Iterate through each line in the file
    for line in reader.lines() {
        let line = line.expect("Could not read line");

        if line.contains("osm_id") {
            // Trim the line and remove unnecessary characters
            let line = line.trim().trim_start_matches("{").trim_end_matches("}");
            let parts: Vec<&str> = line.split(", ").collect();

            // Initialize variables for osm_id, latitude, and longitude
            let mut osm_id = String::new();
            let mut lat = String::new();
            let mut lon = String::new();

            // Parse the attributes
            for part in parts {
                if part.contains("osm_id") {
                    osm_id = part.split(":").nth(1).unwrap().trim().replace("'", "").to_string();
                } else if part.contains("latitude") {
                    lat = part.split(":").nth(1).unwrap().trim().to_string();
                } else if part.contains("longitude") {
                    lon = part.split(":").nth(1).unwrap().trim().to_string();
                }
            }

            // Check if coordinates match
            if lat == latitude && lon == longitude {
                return Some(osm_id); // Return the node ID if found
            }
        }
    }

    None // Return None if no match found
}

fn get_adjacency_nodes(node: i32) -> io::Result<Vec<Edge>> {
    let filename = "output_nodes.txt";
    get_adjacency_nodes_from_file(filename, node_id)
}

fn get_adjacency_nodes_from_file(filename: &str, node_id: &str) -> io::Result<Vec<Edge>> {
    let mut adjacency_nodes = Vec::new();

    // Open the file
    let file = File::open(filename)?;
    let reader = io::BufReader::new(file);

    // Iterate through each line in the file
    for line in reader.lines() {
        let line = line?;

        // Check if the line contains the node data
        if line.contains("osm_id") {
            // Trim the line and remove unnecessary characters
            let line = line.trim().trim_start_matches("{").trim_end_matches("}");
            let parts: Vec<&str> = line.split(", ").collect();

            let mut osm_id = String::new();
            let mut adjacency_list = Vec::new();

            // Parse the attributes
            for part in parts {
                if part.contains("osm_id") {
                    osm_id = part.split(":").nth(1).unwrap().trim().replace("'", "").to_string();
                } else if part.contains("adjacency_list") {
                    adjacency_list = part.split(":").nth(1)
                        .unwrap().trim().replace("'", "")
                        .replace("[", "").replace("]", "")
                        .split(",")
                        .map(|s| s.trim().to_string())
                        .collect();
                }
            }

            // Check if the current node_id matches
            if osm_id == node_id {
                for adjacent_id in adjacency_list {
                    adjacency_nodes.push(Edge {
                        from: osm_id.clone(),
                        to: adjacent_id,
                    });
                }
            }
        }
    }

    // Return the vector of edges
    Ok(adjacency_nodes)
}

fn dijkstra(start: i32, end: i32) -> Vec<i32> {
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

        match get_adjacency_nodes(node) {
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
    let start = 1; // Starting node for Dijkstra's algorithm
    let end = 4; // Destination node

    dijkstra(start, end);

    // let result = get_shortest_path(start_node, destination_node);
    //     println!("{}", result);
}
