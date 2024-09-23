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
    weight: i32,
    from: i32,
    to: i32,
}

// type AdjacencyList = HashMap<i32, Vec<Edge>>;

// fn read_graph_from_file(filename: &str) -> io::Result<AdjacencyList> {
//     let mut adj_list: AdjacencyList = HashMap::new();

//     if let Ok(lines) = read_lines(filename) {
//         for line in lines {
//             if let Ok(edge_line) = line {
//                 let parts: Vec<i32> = edge_line
//                     .split_whitespace()
//                     .filter_map(|x| x.parse().ok())
//                     .collect();

//                 if parts.len() != 3 {
//                     eprintln!("Warning: Skipping malformed line: {}", edge_line);
//                     continue;
//                 }

//                 let u_id = parts[0];
//                 let v_id = parts[1];
//                 let weight = parts[2];

//                 let u_vertex = Vertex { id: u_id };
//                 let v_vertex = Vertex { id: v_id };

//                 let edge_u_to_v = Edge {
//                     weight,
//                     from: u_vertex.clone(),
//                     to: v_vertex.clone(),
//                 };

//                 let edge_v_to_u = Edge {
//                     weight,
//                     from: v_vertex.clone(),
//                     to: u_vertex.clone(),
//                 };

//                 adj_list.entry(u_id).or_insert_with(Vec::new).push(edge_u_to_v);
//                 adj_list.entry(v_id).or_insert_with(Vec::new).push(edge_v_to_u);
//             }
//         }
//     }
//     Ok(adj_list)
// }

fn get_adjacency_nodes(node: i32) -> io::Result<Vec<Edge>> {
    let filename = "graph_edges.txt";
    get_adjacency_nodes_from_file(filename, node)
}

// Function to get adjacency nodes for a given node from a file
fn get_adjacency_nodes_from_file(filename: &str, node: i32) -> io::Result<Vec<Edge>> {
    let mut adjacency_nodes = Vec::new();

    if let Ok(lines) = read_lines(filename) {
        for line in lines {
            if let Ok(edge) = line {
                let parts: Vec<i32> = edge.split_whitespace().map(|x| x.parse().unwrap()).collect();
                if parts.len() == 3 {
                    let u = parts[0];
                    let v = parts[1];
                    let weight = parts[2];

                    if u == node {
                        adjacency_nodes.push(Edge { from: u, to: v, weight });
                    }
                    if v == node {
                        adjacency_nodes.push(Edge { from: v, to: u, weight });
                    }
                }
            }
        }
    }

    // Return the vector of edges
    Ok(adjacency_nodes)
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
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



// fn get_shortest_path(start: i32, end: i32) -> String {

//     // Run Dijkstra's algorithm to get distances and predecessors
//     let (distances, predecessors) = dijkstra(start, end);

//     // Prepare the result string
//     if distances.contains_key(&end) {
//         let distance = distances[&end];
//         let path = reconstruct_path(&predecessors, start, end);
//         format!(
//             "Distance from node {} to node {}: {}\nPath: {:?}",
//             start, end, distance, path
//         )
//     } else {
//         format!("No path found from node {} to node {}", start, end)
//     }
// }

fn main() {
    let start = 1; // Starting node for Dijkstra's algorithm
    let end = 4; // Destination node

    dijkstra(start, end);

    // let result = get_shortest_path(start_node, destination_node);
    //     println!("{}", result);
}
