use aws_sdk_dynamodb as dynamodb;
use dynamodb::{types::AttributeValue, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_dynamo::aws_sdk_dynamodb_1::from_items;
use serde_dynamo::aws_sdk_dynamodb_1::from_item;
use std::collections::HashMap;
use geoutils::{Location, Distance};

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

    /**
     *  aws dynamodb query --table-name osm --index-name CoordinateIndex --key-condition-expression "longitude BETWEEN :lon_min AND :lon_max and latitude BETWEEN :lat_min AND :lat_max" --expression-attribute-values "{\":lat_min\":{\"N\":\"35.0\"}, \":lat_max\":{\"N\":\"45.0\"}, \":lon_min\":{\"N\":\"-80.0\"}, \":lon_max\":{\"N\":\"-70.0\"}}"
     *
     * aws dynamodb query --table-name osm --index-name CoordinateIndex --key-condition-expression "longitude BETWEEN :lon_min AND :lon_max" --filter-expression "latitude BETWEEN :lat_min AND :lat_max" --expression-attribute-values "{\":lat_min\":{\"N\":\"35.0\"}, \":lat_max\":{\"N\":\"45.0\"}, \":lon_min\":{\"N\":\"-80.0\"}, \":lon_max\":{\"N\":\"-70.0\"}}"


     */

    let query_op = client
        .query()
        .table_name(table_name)
        .index_name("CoordinateIndex")
        .key_condition_expression("longitude BETWEEN :lon_min AND :lon_max and latitude BETWEEN :lat_min AND :lat_max")
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
