pub mod dijkstra;
pub mod database;

use aws_sdk_dynamodb as dynamodb;
use dynamodb::{types::AttributeValue, Client};
use std::io::{self};

async fn test_query_node_by_id(client: Client, table_name: String) {
    /**
     * aws dynamodb get-item --consistent-read ^ --table-name osm ^ --key "{\"id\": {\"S\": \"8942477443\"}}"
     *
     * aws dynamodb query ^ --table-name osm ^ --key-condition-expression "id = :node_id" ^ --expression-attribute-values "{\":node_id\": {\"S\": \"8942477443\"}}"
     *
     */

    let node_id = "8942477443";

    let node = database::query_node_by_id(&client, table_name.to_string(), node_id.to_string()).await;

    println!("node: {:?}", node);
}

async fn test_query_node_by_coordinates(client: Client, table_name: String) {
    /**
     *
     *
     * aws dynamodb scan ^ --table-name osm ^ --filter-expression "latitude BETWEEN :lat_min AND :lat_max AND longitude BETWEEN :lon_min AND :lon_max" ^ --expression-attribute-values "{\":lat_min\":{\"N\":\"35.0\"}, \":lat_max\":{\"N\":\"45.0\"}, \":lon_min\":{\"N\":\"-80.0\"}, \":lon_max\":{\"N\":\"-70.0\"}}"


     */

    let longitude = -74.6630728;
    let latitude = 40.3503542;

    let closest_node = database::query_node_by_coordinates(&client, table_name.to_string(), latitude, longitude).await;

    println!("closest node: {:?}", closest_node);




}

async fn test_query_nodes_by_id(client: Client, table_name: String) {
    let node_id = "8942477443";

    let node = database::query_node_by_id(&client, table_name.to_string(), node_id.to_string()).await;

    match node {
        Some(node) => {
            println!("Adjacency List for Node {}: {:?}", node.id, node.adjacency_list);

            let adjacency_nodes  = database::query_nodes_by_id(&client, table_name.to_string(), node.adjacency_list.clone()).await;

            for node in adjacency_nodes {
                println!("Node ID: {}, Latitude: {}, Longitude: {}", node.id.clone(), node.latitude, node.longitude);
            }
        },
        None => {
            println!("No node found.");
        }
    }
}

async fn test_get_shortest_path(client: Client, table_name: String){


    // start
    //  {'osm_id': '103978126', 'latitude': 40.3465896, 'longitude': -74.6646682, 'tags': {}, 'adjacency_list': ['11124620695']}

    // end
    // {'osm_id': '7889842637', 'latitude': 40.3462607, 'longitude': -74.6639481, 'tags': {}, 'adjacency_list': ['11124620695', '6119864729']}

    // Path: ["103978126", "11124620695", "7889842637"]

    let start_lat = 40.3465896;
    let start_lon = -74.6646682;
    let end_lat = 40.3462607;
    let end_lon = -74.6639481;

    get_shortest_path(client, table_name.to_string(), start_lat, start_lon, end_lat, end_lon).await;

}

async fn test_get_shortest_path_multiple(client: Client, table_name: String) {

    // point 1
    //  {'osm_id': '103978126', 'latitude': 40.3465896, 'longitude': -74.6646682, 'tags': {}, 'adjacency_list': ['11124620695']}

    // point 2
    // {'osm_id': '7889842637', 'latitude': 40.3462607, 'longitude': -74.6639481, 'tags': {}, 'adjacency_list': ['11124620695', '6119864729']}

    // point 3
    // {'osm_id': '104052241', 'latitude': 40.3458467, 'longitude': -74.6630416, 'tags': {}, 'adjacency_list': ['6119864729', '104052245']}

    // "103978126", "11124620695", "7889842637", "6119864729", "104052241",

    let points = vec![
        (40.3465896, -74.6646682),
        (40.3462607, -74.6639481),
        (40.3458467, -74.6630416),
    ];

    let path = get_shortest_path_multiple(client, table_name.to_string(), points).await;
    // Print the result of the shortest path
    println!("Shortest path: {:?}", path);

}

async fn get_shortest_path(client: Client, table_name: String, start_lat: f64, start_lon: f64, end_lat: f64, end_lon: f64) -> Vec<String> {

    let src_node = database::query_node_by_coordinates(&client, table_name.to_string(), start_lat, start_lon).await.unwrap();
    let dest_node = database::query_node_by_coordinates(&client, table_name.to_string(), end_lat, end_lon).await.unwrap();

    println!("get_shortest_path: start{}, end{}", src_node.id, dest_node.id);
    dijkstra::dijkstra(&client, table_name.to_string(),src_node, dest_node).await
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
        let src_node = database::query_node_by_coordinates(&client, table_name.clone(), start_lat, start_lon).await.unwrap();
        let dest_node = database::query_node_by_coordinates(&client, table_name.clone(), end_lat, end_lon).await.unwrap();

        println!("get_shortest_path: start {}, end {}", src_node.id, dest_node.id);

        // Calculate the shortest path using dijkstra's algorithm
        let segment_path = dijkstra::dijkstra(&client, table_name.clone(), src_node, dest_node).await;

        // Add the segment path to the overall path
        // if second segment, dont include first node
        if (i == 0) {
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

#[tokio::main]
async fn main() -> io::Result<()> {
    let config = aws_config::load_from_env().await;

    let client = aws_sdk_dynamodb::Client::new(&config);

    let table_name = "osm";

    test_get_shortest_path_multiple(client, table_name.to_string()).await;

    Ok(())
}
