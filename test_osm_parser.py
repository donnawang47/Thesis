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


if __name__ == "__main__":

    osm_file_path = "map_mini.osm"
    osm_data = parse_osm_file(osm_file_path)
    write_osm_data_to_file(osm_data)




