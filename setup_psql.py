import psycopg2
from psycopg2 import sql

import argparse
import logging
import hashlib
import secrets
import time
from uuid import uuid4
import random
from decimal import Decimal

import osmium
import boto3
from botocore.exceptions import ClientError

# Define a handler class to extract nodes, ways, and relations from the .osm file
class OSMHandler(osmium.SimpleHandler):
    def __init__(self):
        osmium.SimpleHandler.__init__(self)
        self.nodes = {}
        self.ways = []
        self.relations = []

    # Method to handle nodes
    def node(self, n):
        try:
            # node dict
            self.nodes[str(n.id)] = {
                "osm_id": str(n.id),
                "latitude": n.location.lat,
                "longitude": n.location.lon,
                "tags": {tag.k: tag.v for tag in n.tags},
                "adjacency_list": []  # Initialize adjacency list
            }
        except Exception as e:
            print(f"Error processing node {n.id}: {e}")

    # Method to handle ways
    def way(self, w):
        try:
            node_refs = [str(n.ref) for n in w.nodes]
            # Build the adjacency list for each node in the way
            for i, node_ref in enumerate(node_refs):
                if node_ref in self.nodes:
                    if not isinstance(self.nodes[node_ref]["adjacency_list"], list):
                        print("not adjacency list")

                    # Add neighboring nodes to the adjacency list
                    if i > 0:  # Add the previous node in the way
                        self.nodes[node_ref]["adjacency_list"].append(node_refs[i - 1])
                    if i < len(node_refs) - 1:  # Add the next node in the way
                        self.nodes[node_ref]["adjacency_list"].append(node_refs[i + 1])
            # Store the way information
            way_item = {
                "osm_id": str(w.id),
                "type": "way",
                "nodes": node_refs,
                "tags": {tag.k: tag.v for tag in w.tags},
            }
            self.ways.append(way_item)

        except Exception as e:
            print(f"Error processing way {w.id}: {e}")

    # Method to handle relations
    def relation(self, r):
        try:
            item = {
                "osm_id": str(r.id),
                "type": "relation",
                "members": [{"type": m.type, "ref": m.ref, "role": m.role} for m in r.members],
                "tags": {tag.k: tag.v for tag in r.tags},
            }
            # Append the relation data to the relations list
            self.relations.append(item)
        except Exception as e:
            print(f"Error processing relation {r.id}: {e}")

    def get_data(self):
        # Print the sizes of the collections
        print(f"Number of nodes: {len(self.nodes)}")
        print(f"Number of ways: {len(self.ways)}")
        print(f"Number of relations: {len(self.relations)}")

        return {
            "nodes": self.nodes,
            "ways": self.ways,
            "relations": self.relations
        }

# Function to read and process the entire .osm file
def parse_osm_file(osm_file_path):
    handler = OSMHandler()
    handler.apply_file(osm_file_path, locations=True)
    print(f"Finished reading and processing file: {osm_file_path}")
    osm_data = handler.get_data()
    return osm_data

def write_osm_data_to_file(osm_data):
    nodes_file = "output_nodes.txt"
    ways_file = "output_ways.txt"
    relations_file = "output_relations.txt"

    try:
        # Write nodes with adjacency list to a separate file
        with open(nodes_file, 'w') as nodes_f:
            nodes_f.write("Nodes with Adjacency List:\n")
            for node_id, node_data in osm_data['nodes'].items():
                nodes_f.write(f"{node_data}\n")
        print(f"Nodes data successfully written to {nodes_file}")

        # Write ways to a separate file
        with open(ways_file, 'w') as ways_f:
            ways_f.write("Ways:\n")
            for way in osm_data['ways']:
                ways_f.write(str(way) + '\n')
        print(f"Ways data successfully written to {ways_file}")

        # Write relations to a separate file
        with open(relations_file, 'w') as relations_f:
            relations_f.write("Relations:\n")
            for relation in osm_data['relations']:
                relations_f.write(str(relation) + '\n')
        print(f"Ways data successfully written to {relations_file}")
    except Exception as e:
        print(f"Error writing OSM data to file: {e}")


def create_database(dbname='osm', user='postgres', password='xxx', host='localhost', port=5432):
    """
    Creates a PostgreSQL database locally.

    Parameters:
    dbname (str): Name of the database to be created.
    user (str): Username for PostgreSQL authentication.
    password (str): Password for PostgreSQL authentication.
    host (str): Host where PostgreSQL is running (default is 'localhost').
    port (int): Port where PostgreSQL is running (default is 5432).

    Returns:
    None
    """
    try:
        # Connect to the default database 'postgres' for database creation and deletion
        connection = psycopg2.connect(
            dbname=dbname,
            user=user,
            password=password,
            host=host,
            port=port
        )
        connection.autocommit = True
        cursor = connection.cursor()

        # Drop the database if it exists
        cursor.execute(sql.SQL("DROP DATABASE IF EXISTS {}").format(
            sql.Identifier(dbname)
        ))
        print(f"Database {dbname} dropped successfully if it existed.")

        # Create the new database
        cursor.execute(sql.SQL("CREATE DATABASE {}").format(
            sql.Identifier(dbname)
        ))
        print(f"Database {dbname} created successfully.")

        # Close the cursor and connection
        cursor.close()
        connection.close()

        # Connect to the new database
        new_connection = psycopg2.connect(
            dbname=dbname,
            user=user,
            password=password,
            host=host,
            port=port
        )
        new_cursor = new_connection.cursor()

        # Create PostGIS extension
        # CREATE EXTENSION postgis;
        # SELECT * FROM pg_extension;

        new_cursor.execute("CREATE EXTENSION IF NOT EXISTS postgis;")
        print("PostGIS extension added successfully.")

        # Close the new cursor and connection
        new_cursor.close()
        new_connection.close()

    except Exception as error:
        print(f"Error creating database: {error}")


def add_table_to_postgres(dbname='osm', user='postgres', password='xxx', host='localhost', port=5432, table_name = None, columns = None):
    """
    Create a table in a PostgreSQL database.

    Parameters:
    - db_name: Name of the database.
    - user: Username for the database.
    - password: Password for the user.
    - host: Host where the database is located.
    - port: Port number for the database.
    - table_name: Name of the table to be created.
    - columns: A dictionary where keys are column names and values are their data types.
    """
    if table_name is None or columns is None:
        print("Error: table_name and columns must be provided.")
        return
    try:
        # Connect to the PostgreSQL database
        connection = psycopg2.connect(
            dbname= dbname,
            user=user,
            password=password,
            host=host,
            port=port
        )
        cursor = connection.cursor()

        # Drop the table if it already exists
        drop_table_query = sql.SQL("DROP TABLE IF EXISTS {}").format(sql.Identifier(table_name))
        cursor.execute(drop_table_query)

        # Construct the CREATE TABLE SQL statement
        column_defs = ', '.join([f"{name} {dtype}" for name, dtype in columns.items()])
        create_table_query = sql.SQL("CREATE TABLE IF NOT EXISTS {} ({})").format(
            sql.Identifier(table_name),
            sql.SQL(column_defs)
        )

        # Execute the SQL statement
        cursor.execute(create_table_query)
        connection.commit()

        print(f"Table '{table_name}' created successfully.")

    except Exception as e:
        print(f"Error occurred: {e}")
    finally:
        # Close the cursor and connection
        if cursor:
            cursor.close()
        if connection:
            connection.close()

def insert_adj_nodes_to_postgres(osm_data, dbname='osm', user='postgres', password='xxx', host='localhost', port=5432):
    """Insert OSM data into the PostgreSQL database."""
    try:
        # Connect to the PostgreSQL database
        connection = psycopg2.connect(
            dbname=dbname,
            user=user,
            password=password,
            host=host,
            port=port
        )
        cursor = connection.cursor()

        for node_id, node_info in osm_data['nodes'].items():

            # Convert the adjacency list to integers
            adjacency_list = [int(n) for n in node_info["adjacency_list"]]

            # Prepare the SQL insert statement with both osm_id and nodes
            insert_query = sql.SQL(
                "INSERT INTO adjacent_nodes (id, nodes) VALUES (%s, %s)"
            )

            # Execute the SQL statement
            cursor.execute(insert_query, (int(node_id), adjacency_list))

        connection.commit()
        print("Data inserted successfully.")

    except Exception as e:
        print(f"Error inserting data into PostgreSQL: {e}")
    finally:
        # Close the cursor and connection
        if cursor:
            cursor.close()
        if connection:
            connection.close()


if __name__ == "__main__":

    osm_file_path = "map_mini.osm"
    osm_data = parse_osm_file(osm_file_path)
    # write_osm_data_to_file(osm_data)

    table_name = "adjacent_nodes"
    columns = {
        "id": "BIGINT PRIMARY KEY",
        "nodes": "BIGINT[]"
    }

    add_table_to_postgres(table_name = table_name, columns = columns)
    insert_adj_nodes_to_postgres(osm_data)
