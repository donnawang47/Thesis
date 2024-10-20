pub mod database;
pub mod dijkstra;

use std::io::{self};
use dotenv::dotenv;
use std::env;

pub fn load_config() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

// pub fn test_query_node_by_coordinates() {
//     // Define latitude and longitude
//     let latitude = 40.3503542;
//     let longitude = -74.6630728;
//     let tolerance = 5.0; // tolerance in meters

//     // Query nodes based on latitude and longitude
//     let nodes = database::query_nodes_by_lat_lon(&pool, latitude, longitude, tolerance).await?;

//     // Print the results
//     for node in nodes {
//         println!("Node ID: {}", node.osm_id);
//     }

//     Ok(())
// }

// async fn test_query_node_by_name() {
//     let point_name = "Kung Fu Tea";
//     match database::get_node_by_name(&pool, point_name).await {
//         Ok(Some(node)) => {
//             println!("Found node: {:?}", node);
//         }
//         Ok(None) => {
//             println!("Node not found.");
//         }
//         Err(err) => {
//             eprintln!("Error occurred: {}", err);
//         }
//     }

// }


/**
 *
 * test deprecated get adj nodes
 *
let node_id = 103981998;
let next_nodes = database::get_adjacent_nodes(&pool, node_id).await?;

for node in next_nodes {
    println!("{:?}", node);
}
 */

 async fn test_get_adj_nodes(pool: &sqlx::PgPool) {
    let node_id: i64 = 103984130;

    database::get_adjacent_nodes(&pool, node_id).await.unwrap_or_else(|e| {
        eprintln!("Error fetching adjacent nodes: {:?}", e);
        vec![] // Return an empty vector in case of error
    })
    .into_iter()
    .for_each(|node| {
        println!("{:?}", node);
    });
}

async fn test_get_node_by_id(pool: &sqlx::PgPool) {
    let node_id: i64 = 103994771;

    match database::get_node_by_id(&pool, node_id).await {
        Ok(node) => println!("{:?}", node),
        Err(e) => eprintln!("Error fetching node: {}", e),
    }
}

/**
 * output

 get_shortest_path: start 103994771, end 8942477433
Distance from node 103994771 to node 8942477433: 47.053000000000004
Path: [103994771, 104105317, 5604208759, 104105315, 5604208758, 104105313, 104105311, 8942477433]
Shortest path found: [103994771, 104105317, 5604208759, 104105315, 5604208758, 104105313, 104105311, 8942477433]

 */
async fn test_get_shortest_path(pool: &sqlx::PgPool) {
    // -74.663318, 40.351712
    let start_lon = -74.663318;
    let start_lat = 40.351712;

    // -74.6633979, 40.3514991
    let end_lon = -74.6633467;
    let end_lat = 40.351305;

    // Call the get_shortest_path function
    match get_shortest_path(&pool, start_lat, start_lon, end_lat, end_lon).await {
        Ok(path) => {
            println!("Shortest path found: {:?}", path);
        },
        Err(e) => {
            eprintln!("Error finding shortest path: {:?}", e);
        }
    }
}

/**
 * output

 get_shortest_path: start 103994771, end 8942477433
Distance from node 103994771 to node 8942477433: 47.053000000000004
Path: [103994771, 104105317, 5604208759, 104105315, 5604208758, 104105313, 104105311, 8942477433]
get_shortest_path: start 8942477433, end 104105303
Distance from node 8942477433 to node 104105303: 89.57499999999999
Path: [8942477433, 104105309, 104105306, 104105303]
Shortest path: [103994771, 104105317, 5604208759, 104105315, 5604208758, 104105313, 104105311, 8942477433, 104105309, 104105306, 104105303]

 */
async fn test_get_shortest_path_multiple(pool: &sqlx::PgPool) {

    // Define your points here
    let points = vec![
        (40.351712, -74.663318), // 103994771
        (40.351305, -74.6633467), // 8942477433
        (40.35054, -74.6630122), // 104105303
    ];

    let path = get_shortest_path_multiple(&pool, points).await;


    // Get the shortest path for multiple points
    match path {
        Ok(path) => {
            println!("Shortest path: {:?}", path);
        }
        Err(e) => {
            eprintln!("Error getting shortest path: {}", e);
        }
    }

}

async fn get_shortest_path(pool: &sqlx::PgPool, start_lat: f64, start_lon: f64, end_lat: f64, end_lon: f64) -> Result<Vec<i64>, io::Error> {
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

    println!("get_shortest_path: start {}, end {}", src_node.id, dest_node.id);

    // Call the dijkstra function and return its result
    let path = dijkstra::dijkstra(pool, src_node, dest_node).await;

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

#[tokio::main]
async fn main() -> io::Result<()> {
    // Load database configuration
    let config = load_config();

    // Create a connection pool
    let pool = database::create_pool(&config).await?;

    /**
     * let longitude = -74.6554635;
    let latitude = 40.3463566;
    let distance_limit = 10.0; // Distance in meters
     */

    // test_get_adj_nodes(&pool).await;
    // test_get_node_by_id(&pool).await;
    // test_get_shortest_path(&pool).await;





    drop(pool); // This will close all connections in the pool
    Ok(())
}
