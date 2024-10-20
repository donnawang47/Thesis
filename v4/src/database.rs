use sqlx::{postgres::PgPoolOptions};
use std::io;
use serde::Deserialize;
use sqlx::FromRow;
use geoutils::{Location, Distance};

#[derive(Debug, Deserialize, FromRow)]
pub struct Node {
    pub osm_id: i64,
    pub longitude: f64,
    pub latitude: f64,
    pub name: Option<String>,
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

/**
 *

SELECT osm_id,
       ST_X(way) AS longitude,
       ST_Y(way) AS latitude,
       name
FROM planet_osm_point
WHERE ST_DWithin(
    way::geography,
    ST_MakePoint(-74.659414, 40.3307543)::geography,
    5.0
);


SELECT osm_id,
       ST_X(way) AS longitude,
       ST_Y(way) AS latitude,
       name
FROM planet_osm_point
WHERE ST_DWithin(
    ST_Point(-74.6603363, 40.3496821),
    10.0
);

 */

 /**
  * query by name
  SELECT
    osm_id,
    ST_X(ST_Transform(way, 4326)) AS longitude,
    ST_Y(ST_Transform(way, 4326)) AS latitude,
    name
FROM
    planet_osm_point
WHERE
    name = 'Kung Fu Tea';
  */

pub async fn get_node_by_name(
    pool: &sqlx::PgPool,
    point_name: &str,
) ->  Result<Option<Node>, io::Error> {
    let query = r#"
        SELECT
            osm_id as "osm_id!",
            ST_X(ST_Transform(way, 4326)) as "longitude!",
            ST_Y(ST_Transform(way, 4326)) as "latitude!",
            name
        FROM
            planet_osm_point
        WHERE
            name = $1  -- Parameterized query, safe from SQL injection
        "#;

    let node = sqlx::query_as::<_, Node>(query)
        .bind(point_name)
        .fetch_one(pool)  // This will fetch a single row
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    Ok(Some(node))
}

/**
 * query by osm id

 SELECT
    osm_id,
    ST_X(way) AS longitude,
    ST_Y(way) AS latitude,
    name  -- Assuming you want to select the name as well, adjust based on your needs
FROM
    planet_osm_point
WHERE
    osm_id = 12152402324;  -- Placeholder for the OSM ID

 */
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

pub async fn query_node_by_coordinates(pool: &sqlx::PgPool,
    latitude: f64,
    longitude: f64) -> Result<Option<RawNode>, io::Error> {

    // Set the latitude and longitude range (+/- 0.0001)
    let delta = 0.0001;
    let lat_min = latitude - delta;
    let lat_max = latitude + delta;
    let lon_min = longitude - delta;
    let lon_max = longitude + delta;

    let query = r#"
        SELECT
            id,
            (longitude / 1e7)::FLOAT8 AS lon,
            (latitude / 1e7)::FLOAT8 AS lat
        FROM
            planet_osm_nodes
        WHERE
            (longitude / 1e7) BETWEEN $1 AND $2
            AND (latitude / 1e7) BETWEEN $3 AND $4
    "#;

    // Execute the query, binding the min and max longitude and latitude values
    let nodes = sqlx::query_as::<_, RawNode>(query)
        .bind(lon_min)
        .bind(lon_max)
        .bind(lat_min)
        .bind(lat_max)
        .fetch_all(pool)
        .await
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    if nodes.is_empty() {
        println!("No nodes found.");
        return Ok(None);
    }

    // Find the closest node from the queried nodes
    let closest_node = find_closest_node(nodes.clone(), latitude, longitude);

    Ok(closest_node)

}

fn find_closest_node(nodes:Vec<RawNode>, target_lat: f64, target_lon: f64) -> Option<RawNode> {
    let target_location = Location::new(target_lat, target_lon);
    let mut closest_node: Option<RawNode> = None;
    let mut closest_distance = f64::MAX; // Initialize with the maximum possible value

    for node in nodes {
        let node_location = Location::new(node.lat, node.lon);
        let distance = target_location.distance_to(&node_location).unwrap().meters(); // Get distance in meters

        if distance < closest_distance {
            closest_distance = distance;
            closest_node = Some(node.clone());
        }
    }

    closest_node.clone()
}


/**
 * query by points

 SELECT
    osm_id,
    ST_X(ST_Transform(way, 4326)) AS longitude,
    ST_Y(ST_Transform(way, 4326)) AS latitude,
    name,
    ST_Distance(ST_Transform(way, 4326), ST_SetSRID(ST_MakePoint(-74.6554635, 40.3463566), 4326)) AS distance
FROM
    planet_osm_point
WHERE
    ST_DWithin(
        ST_Transform(way, 3857),  -- Use the correct SRID for the geometries
        ST_Transform(ST_SetSRID(ST_MakePoint(-74.6554635, 40.3463566), 4326), 3857),  -- Transform the point to match the geometries' SRID
        1 -- Distance in meters, ensure it aligns with the geometry SRID units
    )
ORDER BY
    distance ASC;

 */
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

/**
 * query adjacent nodes

WITH node_position AS (
    SELECT
        w.id,
        w.nodes,
        array_position(w.nodes, 103984130) AS pos  -- Get the position of 103981998
    FROM
        planet_osm_ways AS w
    WHERE
        103984130 = ANY(w.nodes)  -- Ensure the node exists in the nodes array
)

SELECT
    n.*
FROM
    node_position np
JOIN
    planet_osm_nodes AS n ON n.id IN (
        -- Select previous and next nodes
        np.nodes[np.pos - 1],  -- Previous node
        np.nodes[np.pos + 1]   -- Next node
    )
WHERE
    np.pos IS NOT NULL;  -- Ensure we have a valid position
*/
pub async fn _get_adjacent_nodes(pool: &sqlx::PgPool, osm_id: i64) -> Result<Vec<RawNode>, io::Error> {
    let query = r#"
        WITH node_position AS (
            SELECT
                w.id,
                w.nodes,
                array_position(w.nodes, $1) AS pos  -- Get the position of the target node
            FROM
                planet_osm_ways AS w
            WHERE
                $1 = ANY(w.nodes)  -- Ensure the node exists in the nodes array
        )

        SELECT
            n.*
        FROM
            node_position np
        JOIN
            planet_osm_nodes AS n ON n.id IN (
                np.nodes[np.pos - 1],  -- Previous node
                np.nodes[np.pos + 1]   -- Next node
            )
        WHERE
            np.pos IS NOT NULL;  -- Ensure we have a valid position
    "#;

    let rows = sqlx::query_as::<_,RawNode>(
        query
    )
    .bind(osm_id)
    .fetch_all(pool)
    .await
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;

    Ok(rows)

}


/**
 * query adjacency nodes

 WITH adjacent AS (
	SELECT nodes FROM adjacent_nodes WHERE id = 103981998
)
SELECT n.id, n.lat, n.lon, n.tags
FROM planet_osm_nodes n
JOIN adjacent a ON n.id = ANY(a.nodes)

 */
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