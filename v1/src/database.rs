use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

pub mod graph_edges_parser;

pub struct Vertex {
    pub id: i32,
}

pub struct Edge {
    pub weight: i32,
    pub from: i32,
    pub to: i32,
}

const FILEPATH: &str = "C:\\Users\\dwang\\Thesis\\hello_world\\src\\database\\graph_edges.txt";

pub fn get_adjacency_nodes(node: i32) -> io::Result<Vec<Edge>> {
    let filename = FILEPATH;
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
            } else {
                eprintln!("Error reading line from line");
            }
        }
    } else {
        eprintln!("Error reading line from file");
    }

    // Return the vector of edges
    Ok(adjacency_nodes)
}


fn print_adjacency_nodes(adjacency_nodes: &Vec<Edge>) {
    // Loop through each edge and print it
    for edge in adjacency_nodes {
        println!("From node: {}, To node: {}, Weight: {}", edge.from, edge.to, edge.weight);
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


fn main() -> io::Result<()> {
    let filename = FILEPATH;

    // Test the read_lines function
    if let Ok(lines) = read_lines(filename) {
        for (i, line) in lines.enumerate() {
            if let Ok(content) = line {
                println!("Line {}: {}", i + 1, content);
            }
        }
    } else {
        println!("Could not open the file: {}", filename);
    }

    Ok(())
}