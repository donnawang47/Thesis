DROP TABLE IF EXISTS edges;
DROP TABLE IF EXISTS nodes;

CREATE TABLE nodes (
    id BIGINT PRIMARY KEY,
    longitude DOUBLE PRECISION,
    latitude DOUBLE PRECISION
);

CREATE TABLE edges (
    id TEXT,
    osm_id BIGINT,  -- Add this line if you want to include osm_id
    source BIGINT REFERENCES nodes(id),
    target BIGINT REFERENCES nodes(id),
    length REAL,
    foot TEXT,  -- Change to TEXT to accept "Allowed"
    car_forward TEXT,
    car_backward TEXT,
    bike_forward TEXT,
    bike_backward TEXT,
    train TEXT,
    wkt TEXT,
    PRIMARY KEY (id, osm_id, source, target)
);