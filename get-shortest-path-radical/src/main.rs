
use lambda_http::{ run, service_fn, Body, Error, Request, Response};
use lambda_runtime::{tracing};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use sqlx::{postgres::PgPoolOptions};
use std::io;
use sqlx::FromRow;
use geoutils::{Location};

use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

use dotenv::dotenv;
use std::env;
use reqwest::Client;
use std::time::Instant;
use tokio::join;



const CHECK_ENDPOINT: &str = "http://54.196.134.17:8000/check";

// #[derive(Deserialize)]
// struct Request {
//     // command: String,
//     points: Vec<(f64, f64)>,
// }

// #[derive(Serialize)]
// struct Response {
//     path: Vec<i32>,
// }

pub fn load_config() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

async fn function_handler(event: Request, check_client: reqwest::Client) -> Result<Response<Body>, Error> {

    // let points = event.payload.points;
    // parse request body into expected format
    let body = event.body();
    let body_vec: Vec<u8> = body[..].into();
    let body_json: Value = serde_json::from_slice(&body_vec)?;
    // let points = body_json["points"].clone();
    let points: Vec<(f64, f64)> = serde_json::from_value(body_json["points"].clone())?;

    let consistency_handle = tokio::spawn(async move {

        let payload = json!({
            "Id": "test-id",
            "WriteKeys": [],
            "ReadKeys": [],
            "ConsistencyRate": 0.5
        });


        let post_check_endpoint_start = Instant::now();
        let res = check_client.post(CHECK_ENDPOINT)
            .json(&payload)
            .send()
            .await
            .unwrap();

        let resp_json: Value = res.json().await.unwrap();
        println!("check endpoint response time (ms) {:?}", post_check_endpoint_start.elapsed().as_millis());
        println!("check endpoint response {:?}", resp_json);
        resp_json
    });


    let run_function_handle = tokio::spawn(async move {
        let config = load_config();  // Load config in blocking code
        let pool = create_pool(&config).await?;  // Create pool (async if necessary)

        // Run async function to get shortest path
        let result = get_shortest_path_multiple(&pool, points).await;
        result.map(|p| p.into_iter().map(|x| x as i32).collect::<Vec<i32>>())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    });



    let (cc_json, db_result) = join!(consistency_handle, run_function_handle);


    let cc_json = cc_json?;
    let check_result = cc_json["checkResult"].as_bool().unwrap_or(false);

    let path = db_result??;


    // Build the response JSON
    let resp_json = json!({
        "path": path,
        "checkStatus": check_result,
    });

    // Create and return the response
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&resp_json).unwrap().into())
        .map_err(Box::new)?;

    Ok(resp)


}


/**
aws lambda create-function ^ --function-name get-shortest-path ^ --runtime provided.al2023 ^ --role arn:aws:iam::761018850184:role/lambda ^ --handler get-shortest-path.function_handler ^ --zip-file fileb://C:\Users\dwang\Thesis\get-shortest-path\target\lambda\extensions\get-shortest-path.zip

aws lambda invoke --cli-binary-format raw-in-base64-out --function-name get-shortest-path --payload '{ \"points\": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]] }' response.json
cat response.json


cargo lambda invoke --data-ascii '{
  "body": "{\"points\": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]]}","isBase64Encoded": false}'


  cargo lambda invoke --data-ascii '{
    "resource": "/{proxy+}",
    "path": "/path/to/resource",
    "httpMethod": "POST",
    "headers": {
      "Content-Type": "application/json"
    },
    "queryStringParameters": null,
}'  "isBase64Encoded": false40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]]}",
{"statusCode":200,"headers":{"content-type":"application/json"},"multiValueHeaders":{"content-type":["application/json"]},"body":"{\"checkStatus\":true,\"path\":[103994771,104105317,1309241463,104105315,1309241462,104105313,104105311,352542841,104105309,104105306,104105303]}","isBase64Encoded":false}

*/

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let check_client = Client::new();

    // let handler = |event| function_handler(event, &check_client);

    // lambda_runtime::run(service_fn(handler)).await

    run(service_fn(|event: Request| async {
        function_handler(event, check_client.clone(), ).await
    })).await


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
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))
}

pub async fn get_node_by_id(pool: &sqlx::PgPool, osm_id: i64) -> Result<RawNode, io::Error> {
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
            return Err(io::Error::new(io::ErrorKind::NotFound, "Destination node not found"));
        },
        Err(e) => {
            eprintln!("Error fetching destination node: {:?}", e);
            return Err(e);
        }
    };

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