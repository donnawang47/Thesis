pub mod database;

use crate::database::User;

use dotenv::dotenv;
use std::env;
use std::io;
use geoutils::Location;
use serde_json::json;

pub fn load_config() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

async fn test_get_user(pool: &sqlx::PgPool) {
    let username: String = "1".to_string();

    match database::get_user(&pool, &username).await {
        Ok(user) => println!("{:?}", user),
        Err(e) => eprintln!("Error fetching node: {}", e),
    }
}

async fn test_update_user_location(pool: &sqlx::PgPool) {
    let username = "1".to_string();

    // on route -- deviation false
    let new_latitude = 40.351712;
    let new_longitude = -74.663318;

    // not on route -- deviation true
    // let new_latitude = 0.0;
    // let new_longitude = 0.0;

    match database::update_user_location(pool, &username, new_latitude, new_longitude).await {
        Ok(_) => {
            println!("Successfully updated location for user: {}", username);
            // Optionally fetch and print the updated user to verify the change
            match database::get_user(pool, &username).await {
                Ok(user) => println!("Updated user data: {:?}", user),
                Err(e) => eprintln!("Error fetching updated user: {}", e),
            }
        }
        Err(e) => eprintln!("Error updating location: {}", e),
    }
}

/*
[103994771,104105317,1309241463,104105315,1309241462,104105313,104105311,352542841,104105309,104105306,104105303]
 */

 async fn test_update_user_route_node_ids(pool: &sqlx::PgPool) {
    let username = "1".to_string();
    let route_node_ids = vec![103994771,104105317,1309241463,104105315,1309241462,104105313,104105311,352542841,104105309,104105306,104105303]; // Example node IDs

    // Perform the update
    match database::update_user_route_node_ids(pool, &username, route_node_ids.clone()).await {
        Ok(_) => {
            println!("Successfully updated route node IDs for user: {}", &username);

            // Fetch the user to verify the update
            match database::get_user(pool, &username).await {
                Ok(user) => {
                    println!("Updated user data: {:?}", user);

                    // Validate the updated node IDs
                    assert_eq!(
                        user.unwrap().current_route_node_ids,
                        route_node_ids,
                        "Route node IDs do not match expected values."
                    );
                }
                Err(e) => eprintln!("Error fetching updated user: {}", e),
            }
        }
        Err(e) => eprintln!("Error updating route node IDs: {}", e),
    }
}

async fn test_update_user_route_node_coordinates(pool: &sqlx::PgPool) {
    let username = "1".to_string(); // Assuming the user with username "1" exists

    // Define the new route node coordinates (list of coordinates for the route nodes)
    let new_coordinates = vec![
        vec![40.351712, -74.663318], // Node ID: 103994771
        vec![40.35054, -74.6630122], // Node ID: 104105303
        vec![40.35061, -74.663041],  // Node ID: 104105306
        vec![40.350941, -74.663187], // Node ID: 104105309
        vec![40.351348, -74.663366], // Node ID: 104105311
        vec![40.351399, -74.663384], // Node ID: 104105313
        vec![40.351485, -74.663399], // Node ID: 104105315
        vec![40.351579, -74.663392], // Node ID: 104105317
    ];

    // Call the update_user_route_coordinates function
    match database::update_user_route_node_coordinates(pool, &username, new_coordinates).await {
        Ok(_) => {
            println!("Successfully updated route node coordinates for user: {}", username);

            // Optionally fetch and print the updated user to verify the change
            match database::get_user(pool, &username).await {
                Ok(user) => {
                    println!("Updated user data: {:?}", user);
                    // Here, you could also add assertions to check if the updated coordinates are correct.
                },
                Err(e) => eprintln!("Error fetching updated user: {}", e),
            }
        }
        Err(e) => eprintln!("Error updating route node coordinates: {}", e),
    }
}


fn test_is_point_on_segment() {

    // Helper function to create a Location
    fn create_location(lat: f64, lon: f64) -> Location {
        Location::new(lat, lon)
    }

    // Test cases
    let test_cases = vec![
        // Case 1: Point exactly on the segment
        (
            create_location(0.0, 0.0),           // Point
            create_location(0.0, 0.0),           // Start of segment
            create_location(1.0, 1.0),           // End of segment
            true,                                // Expected result
        ),
        // Case 2: Point exactly at the start of the segment
        (
            create_location(0.0, 0.0),
            create_location(0.0, 0.0),
            create_location(1.0, 1.0),
            true,
        ),
        // Case 3: Point exactly at the end of the segment
        (
            create_location(1.0, 1.0),
            create_location(0.0, 0.0),
            create_location(1.0, 1.0),
            true,
        ),
        // Case 4: Point not on the segment
        (
            create_location(2.0, 2.0),
            create_location(0.0, 0.0),
            create_location(1.0, 1.0),
            false,
        ),
        // Case 5: Point on the same line but outside the segment
        (
            create_location(-1.0, -1.0),
            create_location(0.0, 0.0),
            create_location(1.0, 1.0),
            false,
        ),
        // Case 6: Point not collinear with the segment
        (
            create_location(0.5, 1.0),
            create_location(0.0, 0.0),
            create_location(1.0, 1.0),
            false,
        ),
    ];

    // Run tests
    for (i, (point, start, end, expected)) in test_cases.into_iter().enumerate() {
        let result = database::is_point_on_segment(&point, &start, &end);
        match result {
            Ok(actual) => {
                assert_eq!(
                    actual, expected,
                    "Test case {} failed: point {:?} start {:?} end {:?}",
                    i, point, start, end
                );
            }
            Err(e) => {
                panic!(
                    "Test case {} errored: {:?}, point {:?} start {:?} end {:?}",
                    i, e, point, start, end
                );
            }
        }
    }

    println!("All tests passed for is_point_on_segment!");
}

async fn test_get_node_coordinates(pool: &sqlx::PgPool) {
    // List of node IDs to test with
    let node_ids = vec![
        103994771, 104105317, 1309241463, 104105315, 1309241462, 104105313,
        104105311, 352542841, 104105309, 104105306, 104105303,
    ];

    // Call the function to fetch node coordinates
    match database::get_node_coordinates(&pool, node_ids).await {
        Ok(nodes) => {
            // Print the node coordinates
            for node in nodes {
                println!(
                    "Node ID: {}, Latitude: {}, Longitude: {}, Adjacency List: {:?}",
                    node.id, node.lat, node.lon, node.adjacency_list
                );
            }
        }
        Err(e) => eprintln!("Error fetching node coordinates: {}", e),
    }
}


fn test_check_deviation_from_route() {
    // Coordinates representing the route (Node IDs and their corresponding latitudes and longitudes)
    let route_coordinates = json!([
        [40.351712, -74.663318], // Node ID: 103994771
        [40.35054, -74.6630122], // Node ID: 104105303
        [40.35061, -74.663041],  // Node ID: 104105306
        [40.350941, -74.663187], // Node ID: 104105309
        [40.351348, -74.663366], // Node ID: 104105311
        [40.351399, -74.663384], // Node ID: 104105313
        [40.351485, -74.663399], // Node ID: 104105315
        [40.351579, -74.663392], // Node ID: 104105317
    ]);

    // Simulate a user on the route
    let user_on_route = User {
        username: "testuser".to_string(),
        current_location: Some(json!([40.351712, -74.663318])), // User at the first node
        current_route_node_ids: vec![103994771, 104105303, 104105306], // Example node IDs
        current_route_node_coordinates: Some(route_coordinates.clone()),
        updated_at: None,
    };

    let result = database::check_deviation_from_route(&user_on_route);
    assert_eq!(result.unwrap(), false); // User should be on the route
    println!("Test 1 (User on route) passed");

    // Simulate a user deviated from the route
    let user_deviated = User {
        username: "testuser".to_string(),
        current_location: Some(json!([40.352000, -74.664000])), // Coordinates off the route
        current_route_node_ids: vec![103994771, 104105303, 104105306], // Example node IDs
        current_route_node_coordinates: Some(route_coordinates.clone()),
        updated_at: None,
    };

    let result = database::check_deviation_from_route(&user_deviated);
    assert_eq!(result.unwrap(), true); // User has deviated from the route
    println!("Test 2 (User deviated) passed");

    // Case where no route coordinates are provided
    let user_invalid_route = User {
        username: "testuser".to_string(),
        current_location: Some(json!([40.351712, -74.663318])), // User at the first node
        current_route_node_ids: vec![103994771, 104105303, 104105306], // Example node IDs
        current_route_node_coordinates: None, // Empty route
        updated_at: None,
    };

    let result = database::check_deviation_from_route(&user_invalid_route);
    assert!(result.is_err()); // Should return error due to missing route coordinates
    println!("Test 3 (Invalid route coordinates) passed");
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error>{
    // Load database configuration
    let config = load_config();

    // Create a connection pool
    let pool = database::create_pool(&config).await?;

    test_update_user_location(&pool).await;
    // test_update_user_route_node_coordinates(&pool).await;
    // test_update_user_route_node_ids(&pool).await;
    // test_is_point_on_segment();
    // test_get_node_coordinates(&pool).await;
    // test_check_deviation_from_route();

    let username = "1".to_string();

    // Fetch user data
    let user = match database::get_user(&pool, &username).await {
        Ok(user) => {
            println!("Fetched user: {:?}", user);
            user.unwrap()
        }
        Err(e) => {
            eprintln!("Error fetching user: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Failed to fetch user"));
        }
    };

    // Extract user's current location and route coordinates
    let current_location = match &user.current_location {
        Some(location) => location,
        None => {
            eprintln!("User has no current location");
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "User location not found",
            ));
        }
    };

    let current_route_node_coordinates = match &user.current_route_node_coordinates {
        Some(route_coordinates) => route_coordinates,
        None => {
            eprintln!("User has no route node coordinates");
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Route node coordinates not found",
            ));
        }
    };

    // Check if the user has deviated from the route
    let deviation = match database::check_deviation_from_route(&user) {
        Ok(deviation) => deviation,
        Err(e) => {
            eprintln!("Error checking route deviation: {}", e);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Failed to check route deviation",
            ));
        }
    };

    // Print details
    println!("Current location: {:?}", current_location);
    println!("Current route node coordinates: {:?}", current_route_node_coordinates);
    println!("Deviation detected: {:?}", deviation);

    Ok(())

}
