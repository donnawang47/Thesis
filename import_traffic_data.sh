#!/bin/bash

# Database connection details
DB_NAME="osm_manhattan"
DB_USER="postgres"
DB_PASSWORD="xxx"  # Replace with your actual password
DB_HOST="localhost"
DB_PORT="5432"
TABLE_NAME="traffic_data"
CSV_FILE="traffic_data.csv"  # Replace with the correct path to your CSV file

# Export the password for PostgreSQL
export PGPASSWORD=$DB_PASSWORD

# Drop the existing table if it exists
echo "Dropping table '$TABLE_NAME' if it exists..."
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "DROP TABLE IF EXISTS $TABLE_NAME;"

# Recreate the table with both WKT and geometry columns
echo "Recreating table '$TABLE_NAME'..."
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "
CREATE TABLE $TABLE_NAME (
    segmentid INTEGER,
    m INTEGER,
    d INTEGER,
    hh INTEGER,
    yr INTEGER,
    boro TEXT,
    vol NUMERIC,
    wktgeom TEXT,  -- Store WKT geometry as text
    street TEXT,
    fromst TEXT,
    tost TEXT,
    direction TEXT,
    geom GEOMETRY(POINT, 3875)  -- Store geometry in EPSG:3875
);
"

# Import the CSV file into the new table
echo "Importing data from '$CSV_FILE' into table '$TABLE_NAME'..."
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "\
\copy $TABLE_NAME (segmentid, m, d, hh, yr, boro, vol, wktgeom, street, fromst, tost, direction)
FROM '$CSV_FILE'
DELIMITER ',' CSV HEADER;"

# Convert WKT to geometry and transform to EPSG:3875
echo "Converting WKT to geometry and transforming to EPSG:3875..."
psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c "
UPDATE $TABLE_NAME
SET geom = ST_Transform(ST_SetSRID(ST_GeomFromText(wktgeom), 2263), 3875)
WHERE wktgeom IS NOT NULL;
"

# Check for errors
if [ $? -eq 0 ]; then
    echo "Data imported and transformed successfully!"
else
    echo "Error occurred during import or transformation."
fi
