#!/bin/bash

psql -U postgres -d osm -f create_tables.sql
psql -U postgres -d osm -c "COPY nodes FROM STDIN CSV HEADER;" < nodes.csv
psql -U postgres -d osm -c "COPY edges (id, osm_id, source, target, length, foot, car_forward, car_backward, bike_forward, bike_backward, train, wkt)
FROM STDIN CSV HEADER;" < edges.csv