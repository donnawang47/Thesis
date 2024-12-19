use std::io;
use serde::{Serialize, Deserialize};
use serde_json::Value;

use sqlx::{postgres::PgPoolOptions};
use sqlx::FromRow;
use geoutils::{Location, Distance};
use chrono::{NaiveDateTime, Datelike, Timelike};

const DEVIATION_THRESHOLD: f64 = 100.0; // 100 meters (constant threshold)
const DISTANCE_TOLERANCE: f64 = 0.01; // Tolerance for the sum of distances in meters

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub username: String,
    pub current_location: Option<Value>, // JSON value for latitude/longitude
    pub current_route_node_ids: Vec<i32>,
    pub current_route_node_coordinates: Option<Value>, // JSON array of coordinate pairs
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Clone, Debug, Deserialize, FromRow)]
pub struct RawNode {
    pub id: i64,
    pub lon: f64,
    pub lat: f64,
    pub adjacency_list: Vec<i64>
}

// impl<'r> sqlx::Decode<'r, sqlx::Postgres> for Coordinate {
//     fn decode(
//         value: &'r sqlx::postgres::PgValue<'r>,
//     ) -> Result<Self, sqlx::Error> {
//         let (latitude, longitude): (f64, f64) = value.try_get(0)?;
//         Ok(Coordinate {
//             latitude,
//             longitude,
//         })
//     }
// }

// #[derive(Clone, Debug, Serialize, Deserialize, FromRow)]
// pub struct User {
//     pub username: String,
//     pub current_location: Option<Value>, // JSON value for latitude/longitude
//     pub current_route_node_ids: Vec<i32>,
//     pub current_route_node_coordinates: Option<Value>, // JSON value for list of coordinate pairs
//     pub updated_at: Option<NaiveDateTime>,
// }


pub async fn create_pool(database_url: &str) -> Result<sqlx::PgPool, io::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string())) // Convert sqlx::Error to io::Error
}


 /**
  * SELECT *
FROM users
WHERE username = '1';
  */

pub async fn get_user(
    pool: &sqlx::PgPool,
    username: &str,
) -> Result<Option<User>, io::Error> {
    let query = r#"
        SELECT *
        FROM users
        WHERE username = $1;
    "#;

    let user = sqlx::query_as::<_, User>(query)
        .bind(username)
        .fetch_optional(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    Ok(user)
}


pub async fn get_node_coordinates(
    pool: &sqlx::PgPool,
    node_ids: Vec<i32>,
) -> Result<Vec<RawNode>, io::Error>  {
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
        WHERE planet_osm_nodes.id = ANY($1);
    "#;

    let result = sqlx::query_as::<_, RawNode>(query)
        .bind(&node_ids) // Bind the array of node IDs
        .fetch_all(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    Ok(result)
}

pub async fn update_user_location(
    pool: &sqlx::PgPool,
    username: &str,
    latitude: f64,
    longitude: f64,
) -> Result<(), io::Error> {
    let query = r#"
        UPDATE users
        SET current_location = $1::jsonb
        WHERE username = $2;
    "#;

    let location = serde_json::json!([latitude, longitude]);
    println!("Location to update: {:?}", location);


    // Execute the query and log the number of affected rows
    let result = sqlx::query(query)
        .bind(location)
        .bind(username)
        .execute(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    println!("Rows affected: {}", result.rows_affected());

    if result.rows_affected() == 0 {
        eprintln!("No rows updated. Ensure the username exists or the location is different.");
    }

    Ok(())
}

pub async fn update_user_route_node_ids(
    pool: &sqlx::PgPool,
    username: &str,
    route_node_ids: Vec<i32>,
) -> Result<(), io::Error> {
    let query = r#"
        UPDATE users
        SET current_route_node_ids = $1
        WHERE username = $2;
    "#;

    // Execute the query to update the user's route node IDs
    let result = sqlx::query(query)
        .bind(route_node_ids)
        .bind(username)
        .execute(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    // Log the result
    println!("Rows affected: {}", result.rows_affected());

    if result.rows_affected() == 0 {
        eprintln!("No rows updated. Ensure the username exists or the node IDs are correct.");
    }

    Ok(())

}

pub async fn update_user_route_node_coordinates(
    pool: &sqlx::PgPool,
    username: &str,
    route_node_coordinates: Vec<Vec<f64>>,  // List of coordinates (latitude, longitude) for each node
) -> Result<(), io::Error> {
    let query = r#"
        UPDATE users
        SET current_route_node_coordinates = $1::jsonb
        WHERE username = $2;
    "#;

    // Serialize the route node coordinates as a JSON array
    let coordinates_json = serde_json::json!(route_node_coordinates);

    // Execute the query to update the user's route node coordinates
    let result = sqlx::query(query)
        .bind(coordinates_json)
        .bind(username)
        .execute(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    // Log the result
    println!("Rows affected: {}", result.rows_affected());

    if result.rows_affected() == 0 {
        eprintln!("No rows updated. Ensure the username exists or the coordinates are correct.");
    }

    Ok(())
}


fn parse_coordinates(json: &Value) -> Option<Coordinate> {
    json.as_array().and_then(|arr| {
        if arr.len() == 2 {
            let latitude = arr[0].as_f64()?;
            let longitude = arr[1].as_f64()?;
            Some(Coordinate { latitude, longitude })
        } else {
            None
        }
    })
}

fn parse_route_coordinates(json: &Value) -> Option<Vec<Coordinate>> {
    json.as_array().map(|array| {
        array.iter().filter_map(|coord| parse_coordinates(coord)).collect()
    })
}

// fn access_user_data(user: &User) {
//     if let Some(location_json) = &user.current_location {
//         if let Some(location) = parse_coordinates(location_json) {
//             println!("Current Location: {:?}", location);
//         }
//     }

//     if let Some(route_json) = &user.current_route_node_coordinates {
//         if let Some(route) = parse_route(route_json) {
//             println!("Route Coordinates: {:?}", route);
//         }
//     }
// }





/// Checks if a point lies approximately on a line segment by comparing distances.
pub fn is_point_on_segment(
    point: &Location,
    start: &Location,
    end: &Location,
) -> Result<bool, String> {
    // Calculate distances
    let dist_to_start = point.distance_to(start)?.meters();
    let dist_to_end = point.distance_to(end)?.meters();
    let dist_segment = start.distance_to(end)?.meters();

    // Check if the sum of distances is approximately equal to the segment distance
    let sum_distances = dist_to_start + dist_to_end;
    Ok((sum_distances - dist_segment).abs() <= DISTANCE_TOLERANCE)
}


/// Checks if the user has deviated from their route using a collinearity-based method.
pub fn check_deviation_from_route(user: &User) -> Result<bool, io::Error> {
    // Parse the user's current location
    let current_location = user
        .current_location
        .as_ref()
        .and_then(parse_coordinates)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "User location not found"))?;

    // Parse the route coordinates
    let route_coordinates = user
        .current_route_node_coordinates
        .as_ref()
        .and_then(parse_route_coordinates)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Route coordinates not found"))?;

    // Check if there are enough points to form a route
    if route_coordinates.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Route must have at least two points",
        ));
    }

    let user_location = Location::new(current_location.latitude, current_location.longitude);

    // Iterate through each segment in the route
    for i in 0..route_coordinates.len() - 1 {
        let start_location = Location::new(
            route_coordinates[i].latitude,
            route_coordinates[i].longitude,
        );
        let end_location = Location::new(
            route_coordinates[i + 1].latitude,
            route_coordinates[i + 1].longitude,
        );

        // Check if the user is on the segment
        match is_point_on_segment(&user_location, &start_location, &end_location) {
            Ok(true) => return Ok(false), // User is on the route
            Ok(false) => continue,       // Not on this segment, check the next one
            Err(e) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Error checking segment: {}", e),
                ));
            }
        }
    }

    Ok(true) // If no segments contain the user, they've deviated
}
