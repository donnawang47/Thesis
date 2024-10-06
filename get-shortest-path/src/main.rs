use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use serde::{Deserialize, Serialize};

use aws_sdk_dynamodb as dynamodb;
use dynamodb::{types::AttributeValue, Client};
use serde_dynamo::aws_sdk_dynamodb_1::from_items;
use serde_dynamo::aws_sdk_dynamodb_1::from_item;
use geoutils::{Location};

use std::collections::{HashMap,BinaryHeap};
use std::cmp::Ordering;

/// This is a made-up example. Requests come into the runtime as unicode
/// strings in json format, which can map to any structure that implements `serde::Deserialize`
/// The runtime pays no attention to the contents of the request payload.
#[derive(Deserialize)]
struct Request {
    // command: String,
    points: Vec<(f64, f64)>,
}

/// This is a made-up example of what a response structure may look like.
/// There is no restriction on what it can be. The runtime requires responses
/// to be serialized into json. The runtime pays no attention
/// to the contents of the response payload.
#[derive(Serialize)]
struct Response {
    // req_id: String,
    // msg: String,
    path: Vec<String>,
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    // // Extract some useful info from the request
    // let command = event.payload.command;

    // // Prepare the response
    // let resp = Response {
    //     req_id: event.context.request_id,
    //     msg: format!("Command {}.", command),
    // };

    // // Return `Response` (it will be serialized to JSON automatically by the runtime)
    // Ok(resp)

    let config = aws_config::load_from_env().await;
    let client = aws_sdk_dynamodb::Client::new(&config);
    let table_name = "osm".to_string();

    let points = event.payload.points;

    let path = get_shortest_path_multiple(client, table_name, points).await;

    let resp = Response { path };

    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    /**
     * cargo lambda invoke --data-ascii '{ \"points\": [[40.3465896, -74.6646682], [40.3462607, -74.6639481], [40.3458467, -74.6630416]] }'
     */
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub longitude: f64,
    pub latitude: f64,
    pub adjacency_list: Vec<String>,
}

pub async fn query_node_by_id(
    client: &Client,
    table_name: String,
    node_id: String,
)  -> Option<Node> {


    let query_op = client
        .query()
        .table_name(table_name)
        .key_condition_expression("id = :node_id") // partition key = value (value placeholder :node_id)
        .expression_attribute_values(":node_id", AttributeValue::S(node_id.to_string())) // specify key value pair to pass in
        .send()
        .await;

    match query_op {
        Ok(output) => {
            // Print the output if the query was successful
            if let Some(items) = output.items {
                if let Some(item) = items.get(0) {
                    let node: Node = from_item(item.clone()).unwrap();
                    return Some(node.clone());
                } else {
                    println!("No items found.");
                }
            } else {
                println!("No items found.");
            }
        }
        Err(err) => {
            // Print the error if the query failed
            println!("Error querying items: {}", err);
        }
    }

    None
}

pub async fn query_nodes_by_id(client: &Client,
    table_name: String,
    node_ids: Vec<String>) -> Vec<Node> {

    let mut nodes: Vec<Node> = Vec::new();

    for node_id in node_ids {
        if let Some(node) = query_node_by_id(client, table_name.clone(), node_id).await { // Await the query_node_by_id
            nodes.push(node.clone()); // Push the node if found
            println!("node: {:?}", node);
        }
    }
    nodes
}

pub async fn query_node_by_coordinates(client: &Client,
    table_name: String, latitude: f64, longitude: f64) -> Option<Node> {

    // Set the latitude and longitude range (+/- 0.0001)
    let delta = 0.0001;
    let lat_min = latitude - delta;
    let lat_max = latitude + delta;
    let lon_min = longitude - delta;
    let lon_max = longitude + delta;

    // Use Query instead of Scan for efficiency
    let query_op = client
    .scan()
    .table_name(table_name)
    .filter_expression("latitude BETWEEN :lat_min AND :lat_max AND longitude BETWEEN :lon_min AND :lon_max")
    .expression_attribute_values(":lat_min", AttributeValue::N(lat_min.to_string()))
    .expression_attribute_values(":lat_max", AttributeValue::N(lat_max.to_string()))
    .expression_attribute_values(":lon_min", AttributeValue::N(lon_min.to_string()))
    .expression_attribute_values(":lon_max", AttributeValue::N(lon_max.to_string()))
    .send()
    .await;

    match query_op {
        Ok(output) => {
            if let Some(items) = output.items {
                let nodes: Vec<Node> = from_items(items).unwrap();
                println!("Got {} nodes", nodes.len());

                // Find the closest node from the queried nodes
                let closest_node = find_closest_node(nodes.clone(), latitude, longitude);
                return closest_node;
            } else {
                println!("No items found.");
            }
        }
        Err(err) => {
            println!("Error querying items: {}", err);
        }
    }

    None
}

pub async fn scan_node_by_coordinates(client: &Client,
    table_name: String,latitude: f64, longitude: f64) -> Option<Node> {

    // Set the latitude and longitude range (+/- 5)
    let delta = 0.0001;
    let lat_min = latitude - delta;
    let lat_max = latitude + delta;
    let lon_min = longitude - delta;
    let lon_max = longitude + delta;

    let scan_op = client
    .scan()
    .table_name(table_name)
    .filter_expression("latitude BETWEEN :lat_min AND :lat_max AND longitude BETWEEN :lon_min AND :lon_max")
    .expression_attribute_values(":lat_min", AttributeValue::N(lat_min.to_string()))
    .expression_attribute_values(":lat_max", AttributeValue::N(lat_max.to_string()))
    .expression_attribute_values(":lon_min", AttributeValue::N(lon_min.to_string()))
    .expression_attribute_values(":lon_max", AttributeValue::N(lon_max.to_string()))
    .send()
    .await;

    match scan_op {
        Ok(output) => {
            if let Some(items) = output.items {
                let nodes: Vec<Node> = from_items(items).unwrap();
                println!("Got {} nodes", nodes.len());
                let closest_node = find_closest_node(nodes.clone(), latitude, longitude);

                return closest_node;
            } else {
                println!("No items found.");
            }
        }
        Err(err) => {
            println!("Error scanning items: {}", err);
        }
    }

    None
}

fn find_closest_node(nodes:Vec<Node>, target_lat: f64, target_lon: f64) -> Option<Node> {
    let target_location = Location::new(target_lat, target_lon);
    let mut closest_node: Option<Node> = None;
    let mut closest_distance = f64::MAX; // Initialize with the maximum possible value

    for node in nodes {
        let node_location = Location::new(node.latitude, node.longitude);
        let distance = target_location.distance_to(&node_location).unwrap().meters(); // Get distance in meters

        if distance < closest_distance {
            closest_distance = distance;
            closest_node = Some(node.clone());
        }
    }

    closest_node.clone()
}

async fn get_adjacency_nodes_from_id(client: &Client,
    table_name: String,
    node_id: String,) -> Vec<Node> {

    let node = query_node_by_id(&client, table_name.to_string(), node_id.to_string()).await;
    match node {
        Some(node) => {
            println!("Adjacency List for Node {}: {:?}", node.id, node.adjacency_list);

            let adjacency_nodes  = query_nodes_by_id(&client, table_name.to_string(), node.adjacency_list.clone()).await;

            adjacency_nodes
        },
        None => {
            Vec::new()
        }
    }
}



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
            let next_node = query_node_by_id(&client, table_name.to_string(), next_node_id.to_string()).await.unwrap();

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


async fn get_shortest_path_multiple(client: Client, table_name: String, points: Vec<(f64, f64)>) -> Vec<String> {

    // Check if there are at least two points to form a path
    if points.len() < 2 {
        println!("At least two points are required to calculate a path.");
        return Vec::new();
    }

    let mut path: Vec<String> = Vec::new();

    // Iterate through the points
    for i in 0..points.len() - 1 {
        let (start_lat, start_lon) = points[i];
        let (end_lat, end_lon) = points[i + 1];

        // Query the source and destination nodes
        let src_node = query_node_by_coordinates(&client, table_name.clone(), start_lat, start_lon).await.unwrap();
        let dest_node = query_node_by_coordinates(&client, table_name.clone(), end_lat, end_lon).await.unwrap();

        println!("get_shortest_path: start {}, end {}", src_node.id, dest_node.id);

        // Calculate the shortest path using dijkstra's algorithm
        let segment_path = dijkstra(&client, table_name.clone(), src_node, dest_node).await;

        // Add the segment path to the overall path
        // if second segment, dont include first node
        if i == 0 {
            path.extend(segment_path);
        }
        else {
            // Skip the first node of the segment_path
            if segment_path.len() > 1 {
                path.extend(segment_path[1..].to_vec()); // Use a range to skip the first node
            }
        }

    }

    path


}