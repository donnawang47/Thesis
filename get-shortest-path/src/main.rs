pub mod database;
pub mod dijkstra;

use lambda_http::{ run, service_fn, Body, Error, Request, Response};
use lambda_runtime::{tracing};

use serde_json::{json, Value};

use std::io;

use dotenv::dotenv;
use std::env;
use reqwest::Client;


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


    let config = load_config();  // Load config in blocking code
    let pool = database::create_pool(&config).await?;  // Create pool (async if necessary)

    // Run async function to get shortest path
    let result = get_shortest_path_multiple(&pool, points).await;

    // Map the result to a Vec<[f64; 2]> or handle the error appropriately
    let path = result
        .map(|p| p.into_iter().map(|x| x as [f64; 2]).collect::<Vec<[f64; 2]>>())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;


    // Build the response JSON
    let resp_json = json!({
        "path": path
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

aws lambda invoke --cli-binary-format raw-in-base64-out --function-name get-shortest-path --payload '{"httpMethod": "POST", "body": "{\"points\": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]]}"}' response.json
cat response.json

curl -X POST https://7s3zmqzaknmyvk6wtrn3q5hjaa0pgvsl.lambda-url.us-east-1.on.aws/ \
-H "Content-Type: application/json" \
-d '{"points": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]]}'


curl -X POST http://localhost:9000/ \
-H "Content-Type: application/json" \
-d '{"points": [[40.351712, -74.663318], [40.351305, -74.6633467], [40.35054, -74.6630122]]}'


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



async fn get_shortest_path(pool: &sqlx::PgPool, start_lat: f64, start_lon: f64, end_lat: f64, end_lon: f64) ->  Result<Vec<[f64; 2]>, io::Error> {
    // Fetch the source node
    let src_node = match database::get_node_by_lat_lon(pool, start_lat, start_lon).await {
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
    let dest_node = match database::get_node_by_lat_lon(pool, end_lat, end_lon).await {
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

    // println!("get_shortest_path: start {}, end {}", src_node.id, dest_node.id);

    // Call the dijkstra function and return its result
    let path = dijkstra::dijkstra(pool, src_node, dest_node).await;

    Ok(path) // Return the path
}

async fn get_shortest_path_multiple(
    pool: &sqlx::PgPool,
    points: Vec<(f64, f64)>
) -> Result<Vec<[f64; 2]>, io::Error> {

    if points.len() < 2 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "At least two points are required to calculate a path."));
    }

    let mut full_path: Vec<[f64; 2]> = Vec::new();

    // Loop through each consecutive pair of points
    for i in 0..points.len() - 1 {
        let (start_lat, start_lon) = points[i];
        let (end_lat, end_lon) = points[i + 1];

        // Get the shortest path between the current pair of points (coordinates)
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