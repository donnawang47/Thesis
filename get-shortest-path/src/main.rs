
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};

use serde::{Deserialize, Serialize};

use sqlx::{postgres::PgPoolOptions};
use std::io;
use sqlx::FromRow;
use geoutils::{Location};

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

use dotenv::dotenv;
use std::env;

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
    path: Vec<i32>,
}

pub fn load_config() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

/// This is the main body for the function.
/// Write your code inside it.
/// There are some code example in the following URLs:
/// - https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples
/// - https://github.com/aws-samples/serverless-rust-demo/
async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {

    // Extract some useful info from the request
    // let command = event.payload.command;

    // Prepare the response
    // let resp = Response {
    //     req_id: event.context.request_id,
    //     msg: format!("Command {}.", command),
    // };

    // Return `Response` (it will be serialized to JSON automatically by the runtime)
    // Ok(resp)


    let config = load_config();
    let pool = create_pool(&config).await?;

    let points = event.payload.points;

    let result = get_shortest_path_multiple(&pool, points).await;

    let path = match result {
        Ok(p) => p.into_iter().map(|x| x as i32).collect(), // Convert Vec<i64> to Vec<i32> if necessary
        Err(e) => {
            // Handle error (log it, return it, etc.)
            return Err(Box::new(e));
        }
    };

    let resp = Response { path };

    Ok(resp)


}


/**
aws lambda create-function ^ --function-name get-shortest-path ^ --runtime provided.al2023 ^ --role arn:aws:iam::761018850184:role/lambda ^ --handler get-shortest-path.function_handler ^ --zip-file fileb://C:\Users\dwang\Thesis\get-shortest-path\target\lambda\extensions\get-shortest-path.zip

aws lambda invoke ^ --cli-binary-format raw-in-base64-out ^ --function-name get-shortest-path ^ --cli-binary-format raw-in-base64-out ^ --payload '{ \"points\": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]] }' ^ response.json


cargo lambda invoke --data-ascii '{ \"points\": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]] }'

*/

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}


#[derive(Clone, Debug, Deserialize, FromRow)]
pub struct RawNode {
    pub id: i64,
    pub lon: f64,
    pub lat: f64,
    pub adjacency_list: Vec<i64>
}

pub async fn create_pool(database_url: &str) -> Result<sqlx::PgPool, io::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string())) // Convert sqlx::Error to io::Error
}


// Function to fetch node details from planet_osm_point table
pub async fn get_node_by_id(pool: &sqlx::PgPool, osm_id: i64) -> Result<RawNode, io::Error> {
    // SQL query to select osm_id, longitude, latitude, and name from planet_osm_point
    let query = r#"
        SELECT
            planet_osm_nodes.id,
            (planet_osm_nodes.lon / 1e7)::FLOAT8 AS lon,
            (planet_osm_nodes.lat / 1e7)::FLOAT8 AS lat,
            adjacent_nodes.nodes AS adjacency_list
        FROM
            planet_osm_nodes
        JOIN
            adjacent_nodes ON planet_osm_nodes.id = adjacent_nodes.id
        WHERE
            planet_osm_nodes.id = $1;
    "#;

    // Execute the query and bind the OSM ID
    let node = sqlx::query_as::<_, RawNode>(query)
        .bind(osm_id)
        .fetch_one(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    // Return the node details
    Ok(node)
}

pub async fn get_node_by_lat_lon(
    pool: &sqlx::PgPool,
    latitude: f64,
    longitude: f64
) -> Result<Option<RawNode>, io::Error> {
    let tolerance = 5.0;

    let query = r#"
        WITH nearest_point AS (
            SELECT
                osm_id,
                ST_X(ST_Transform(way, 4326)) AS longitude,
                ST_Y(ST_Transform(way, 4326)) AS latitude
            FROM
                planet_osm_point
            WHERE
                ST_DWithin(
                    ST_Transform(way, 3857),
                    ST_Transform(ST_SetSRID(ST_MakePoint($1, $2), 4326), 3857),
                    1000.0  -- Set your desired distance threshold in meters
                )
            ORDER BY
                ST_Distance(ST_Transform(way, 4326), ST_SetSRID(ST_MakePoint($1, $2), 4326))
            LIMIT 1
        )
        SELECT
            n.id,
            (n.lon / 1e7)::FLOAT8 AS lon,
            (n.lat / 1e7)::FLOAT8 AS lat,
            a.nodes AS adjacency_list
        FROM
            planet_osm_nodes n
        JOIN
            nearest_point np ON n.id = np.osm_id
        JOIN
            adjacent_nodes a ON a.id = np.osm_id;  -- Join with the adjacent_nodes table
    "#;

    let node = sqlx::query_as::<_, RawNode>(query)
        .bind(longitude)
        .bind(latitude)
        .bind(tolerance)
        .fetch_one(pool)  // This will fetch a single row
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    Ok(Some(node))

}



pub async fn get_adjacent_nodes(pool: &sqlx::PgPool, osm_id: i64) -> Result<Vec<RawNode>, io::Error>{
    // Prepare the SQL query
    let query = r#"
        WITH adjacent AS (
            SELECT nodes FROM adjacent_nodes WHERE id = 103994771
        )
        SELECT
            n.id,
            (n.lon / 1e7)::FLOAT8 AS lon,
            (n.lat / 1e7)::FLOAT8 AS lat,
            a.nodes AS adjacency_list
        FROM
            planet_osm_nodes n
        JOIN
            adjacent a ON n.id = ANY(a.nodes);

    "#;

    let nodes = sqlx::query_as::<_, RawNode>(query)
        .bind(osm_id)
        .fetch_all(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    // Return the adjacency list
    Ok(nodes)
}



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

fn reconstruct_path(predecessors: &HashMap<i64, i64>, start: i64, end: i64) -> Vec<i64> {
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
    path
}

fn get_distance(node_a: &RawNode, node_b: &RawNode) -> f64 {
    let node_a_location = Location::new(node_a.lat, node_a.lon);
    let node_b_location = Location::new(node_b.lat, node_b.lon);

    let distance = node_a_location.distance_to(&node_b_location).unwrap().meters();

    distance
}

pub async fn dijkstra(pool: &sqlx::PgPool, src_node: RawNode, dest_node: RawNode) -> Vec<i64> {
    // key = id, val = distance
    let mut distances: HashMap<i64, f64> = HashMap::new();
    // key = id, val = id
    let mut predecessors: HashMap<i64, i64> = HashMap::new();
    let mut heap = BinaryHeap::new();


    distances.insert(src_node.id, 0.0);
    heap.push(State {
        cost: 0.0,
        node: src_node.clone(),
    });

    while let Some(State { cost, node }) = heap.pop() {
        if node.id.to_string() == dest_node.id.to_string() {
            break;
        }

        if cost > *distances.get(&node.id).unwrap_or(&f64::MAX) {
            continue;
        }

        for next_node_id in node.adjacency_list.clone() {
            let next_node = get_node_by_id(&pool, next_node_id).await.unwrap();

            let weight = get_distance(&node, &next_node);
            let next_cost = cost + weight;

            if next_cost < *distances.get(&next_node.id).unwrap_or(&f64::MAX) {
                distances.insert(next_node.id, next_cost);
                predecessors.insert(next_node.id, node.id);
                heap.push(State {
                    cost: next_cost,
                    node: next_node,
                });
            }
        }
    }

    let path = if distances.contains_key(&dest_node.id) {
        let path = reconstruct_path(&predecessors, src_node.id, dest_node.id);
        path
    } else {
        // Print the error message
        println!("No path found from node {} to node {}", src_node.id.to_string(), dest_node.id.to_string());
        Vec::new()
    };

    path

}

async fn get_shortest_path(pool: &sqlx::PgPool, start_lat: f64, start_lon: f64, end_lat: f64, end_lon: f64) -> Result<Vec<i64>, io::Error> {
    // Fetch the source node
    let src_node = match get_node_by_lat_lon(pool, start_lat, start_lon).await {
        Ok(Some(node)) => node,
        Ok(None) => {
            eprintln!("Source node not found");
            return Err(io::Error::new(io::ErrorKind::NotFound, "Source node not found")); // Return an io error
        },
        Err(e) => {
            eprintln!("Error fetching source node: {:?}", e);
            return Err(e); // Propagate the io error
        }
    };

    // Fetch the destination node
    let dest_node = match get_node_by_lat_lon(pool, end_lat, end_lon).await {
        Ok(Some(node)) => node,
        Ok(None) => {
            eprintln!("Destination node not found");
            return Err(io::Error::new(io::ErrorKind::NotFound, "Destination node not found")); // Return an io error
        },
        Err(e) => {
            eprintln!("Error fetching destination node: {:?}", e);
            return Err(e); // Propagate the io error
        }
    };

    // Call the dijkstra function and return its result
    let path = dijkstra(pool, src_node, dest_node).await;

    Ok(path) // Return the path
}

async fn get_shortest_path_multiple(
    pool: &sqlx::PgPool,
    points: Vec<(f64, f64)>
) -> Result<Vec<i64>, io::Error> {

    if points.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "At least two points are required to calculate a path."));
    }

    let mut full_path: Vec<i64> = Vec::new();

    // Loop through each consecutive pair of points
    for i in 0..points.len() - 1 {
        let (start_lat, start_lon) = points[i];
        let (end_lat, end_lon) = points[i + 1];

        // Get the shortest path between the current pair of points
        let segment_path = get_shortest_path(pool, start_lat, start_lon, end_lat, end_lon).await?;

        if i == 0 {
            // For the first segment, include the entire path
            full_path.extend(segment_path);
        } else if segment_path.len() > 1 {
            // For subsequent segments, skip the first node to avoid duplication
            full_path.extend(segment_path[1..].to_vec());
        }
    }

    Ok(full_path)
}