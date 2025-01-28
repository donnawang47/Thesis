pub mod database;
pub mod dijkstra;

use lambda_http::{run, service_fn, Body, Error, Request, Response};
use serde_json::{json, Value};
use std::io;
use dotenv::dotenv;
use std::env;
use reqwest::Client;
use std::time::{Duration,Instant};
use chrono::{Utc, DateTime};
use lambda_runtime::tracing::{info, debug, error};

// Load configuration from environment
pub fn load_config() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

// The function handler for Lambda
async fn function_handler(event: Request, check_client: reqwest::Client) -> Result<Response<Body>, Error> {
    fn log_event(event_name: &str, start: Instant) {
        info!("{} completed in {:?}", event_name, start.elapsed());
    }

    let timeout_threshold = Duration::from_secs(10); // Define the timeout threshold

    let start_time = Instant::now();
    info!("Function execution started at {}", Utc::now().to_rfc3339());

    // Parse request body
    let body_parse_start = Instant::now();
    let body = event.body();
    let body_vec: Vec<u8> = body[..].into();
    let body_json: Value = serde_json::from_slice(&body_vec)?;
    let points: Vec<(f64, f64)> = serde_json::from_value(body_json["points"].clone())?;
    log_event("Body parsing", body_parse_start);

    // Load configuration
    let config_load_start = Instant::now();
    let config = load_config();
    log_event("Config loading", config_load_start);

    // Create database pool
    let pool_create_start = Instant::now();
    let pool = database::create_pool(&config).await?;
    log_event("Database pool creation", pool_create_start);

    // Get the shortest path
    let path_calc_start = Instant::now();
    let result = get_shortest_path_multiple(&pool, points).await;
    log_event("Path calculation", path_calc_start);

    // Map the result to a Vec<[f64; 2]>
    let path = result
        .map(|p| p.into_iter().map(|x| x as [f64; 2]).collect::<Vec<[f64; 2]>>())
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

    // Build the response JSON
    let response_build_start = Instant::now();
    let resp_json = json!({ "path": path, "timeout": start_time.elapsed() > timeout_threshold});
    let resp = Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&resp_json).unwrap().into())
        .map_err(Box::new)?;
    log_event("Response building", response_build_start);

    info!(
        "Total function execution time: {:?}",
        start_time.elapsed()
    );

    Ok(resp)
}

// The entry point for Lambda
#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    dotenv().ok();

    lambda_runtime::tracing::init_default_subscriber();


    // Log function start
    info!("Function execution started");

    let check_client = reqwest::Client::new();

    run(service_fn(|event: Request| async {
        function_handler(event, check_client.clone()).await
    }))
    .await
}

async fn get_shortest_path(
    pool: &sqlx::PgPool,
    start_lat: f64,
    start_lon: f64,
    end_lat: f64,
    end_lon: f64,
) -> Result<Vec<[f64; 2]>, io::Error> {
    let src_start_time = Instant::now();
    let src_node = match database::get_node_by_lat_lon(pool, start_lat, start_lon).await {
        Ok(Some(node)) => {
            info!("Source node fetched in {:?}", src_start_time.elapsed());
            node
        }
        Ok(None) => {
            error!("Source node not found");
            return Err(io::Error::new(io::ErrorKind::NotFound, "Source node not found"));
        }
        Err(e) => {
            error!("Error fetching source node: {:?}", e);
            return Err(e);
        }
    };

    let dest_start_time = Instant::now();
    let dest_node = match database::get_node_by_lat_lon(pool, end_lat, end_lon).await {
        Ok(Some(node)) => {
            info!("Destination node fetched in {:?}", dest_start_time.elapsed());
            node
        }
        Ok(None) => {
            error!("Destination node not found");
            return Err(io::Error::new(io::ErrorKind::NotFound, "Destination node not found"));
        }
        Err(e) => {
            error!("Error fetching destination node: {:?}", e);
            return Err(e);
        }
    };

    let path_start_time = Instant::now();
    let path = dijkstra::dijkstra(pool, src_node, dest_node).await;
    info!("Path calculation completed in {:?}", path_start_time.elapsed());

    Ok(path)
}

async fn get_shortest_path_multiple(
    pool: &sqlx::PgPool,
    points: Vec<(f64, f64)>,
) -> Result<Vec<[f64; 2]>, io::Error> {
    if points.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "At least two points are required to calculate a path.",
        ));
    }

    let mut full_path: Vec<[f64; 2]> = Vec::new();

    for i in 0..points.len() - 1 {
        let (start_lat, start_lon) = points[i];
        let (end_lat, end_lon) = points[i + 1];

        let segment_start_time = Instant::now();
        let segment_path = get_shortest_path(pool, start_lat, start_lon, end_lat, end_lon).await?;
        info!(
            "Segment {}-{} completed in {:?}",
            i,
            i + 1,
            segment_start_time.elapsed()
        );

        if i == 0 {
            full_path.extend(segment_path);
        } else if segment_path.len() > 1 {
            full_path.extend(segment_path[1..].to_vec());
        }
    }

    Ok(full_path)
}
