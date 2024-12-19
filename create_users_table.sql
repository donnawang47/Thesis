-- psql -U postgres -d osm
-- \i create_users_table.sql

-- Drop the table if it exists
DROP TABLE IF EXISTS users;

-- Drop the custom type if it exists
DROP TYPE IF EXISTS coordinate;

-- Create the custom type for coordinates (latitude, longitude)
CREATE TYPE coordinate AS (
    latitude FLOAT8,
    longitude FLOAT8
);

-- Create the users table
-- CREATE TABLE users (
--     username VARCHAR(50) PRIMARY KEY,
--     current_location coordinate,  -- Use custom type for location (latitude, longitude)
--     current_route_node_ids INTEGER[] DEFAULT '{}',  -- Array of route node IDs
--     current_route_node_coordinates coordinate[] DEFAULT '{}',  -- Array of coordinates
--     updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP  -- Timestamp for when the user was last updated
-- );

CREATE TABLE users (
    username VARCHAR(50) PRIMARY KEY,
    current_location JSONB,  -- JSONB type for location (latitude, longitude)
    current_route_node_ids INTEGER[] DEFAULT '{}',  -- Array of route node IDs
    current_route_node_coordinates JSONB,  -- JSONB for array of coordinates
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP  -- Timestamp for last update
);


-- Insert a sample record into the users table
INSERT INTO users (username)
VALUES ('1');

