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
        self.nodes = []
        self.ways = []
        self.relations = []

    # Method to handle nodes
    def node(self, n):
        try:
            item = {
                "osm_id": str(n.id),
                "type": "node",
                "coordinates": {
                    "latitude": n.location.lat,
                    "longitude": n.location.lon,
                },
                "tags": {tag.k: tag.v for tag in n.tags},
            }
            # Append the node data to the nodes list
            self.nodes.append(item)
        except Exception as e:
            print(f"Error processing node {n.id}: {e}")

    # Method to handle ways
    def way(self, w):
        try:
            item = {
                "osm_id": str(w.id),
                "type": "way",
                "nodes": [{"latitude": n.lat, "longitude": n.lon} for n in w.nodes],
                "tags": {tag.k: tag.v for tag in w.tags},
            }
            # Append the way data to the ways list
            self.ways.append(item)
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


class OSM:
    def __init__(self, dyn_resource):
        self.dyn_resource = dyn_resource
        self.table_name = "osm"
        self.table = None
    def create_table(self):
        try:
            self.table = self.dyn_resource.create_table(
                TableName=self.table_name,
                # define partition key
                KeySchema=[
                    {
                        'AttributeName': 'id',
                        'KeyType': 'HASH'
                    }
                ],
                AttributeDefinitions=[
                    {
                        'AttributeName': 'id',
                        'AttributeType': 'S' # string type
                    }
                ],
                ProvisionedThroughput={
                    'ReadCapacityUnits': 10,
                    'WriteCapacityUnits': 10
                }
            )
        except ClientError as err:
            logger.error(
                "Couldn't create table %s. Here's why: %s: %s",
                self.table_name,
                err.response["Error"]["Code"],
                err.response["Error"]["Message"],
            )
            raise
        else:
            return self.table

    # Insert data into DynamoDB
    def insert_item(self, item):
        try:
            # Put the item into the DynamoDB table
            response = table.put_item(Item=item)
            print(f"Successfully inserted: {item['osm_id']}")
        except ClientError as err:
            logger.error(
                "Couldn't add item to table %s. Here's why: %s: %s",
                self.table_name,
                err.response["Error"]["Code"],
                err.response["Error"]["Message"],
            )
            raise

    # Insert lists of nodes, ways, and relations into DynamoDB
    def insert_osm_data(self, nodes, ways, relations):
        # Insert nodes
        for node in nodes:
            self.insert_item(node)

        # Insert ways
        for way in ways:
            self.insert_item(way)

        # Insert relations
        for relation in relations:
            self.insert_item(relation)

def list_tables(dyn_resource):
    tables = []
    try:
        for table in dyn_resource.tables.all():
            tables.append(table.name)
    except ClientError as err:
        logger.error(
            "Couldn't list tables. Here's why: %s: %s",
            err.response["Error"]["Code"],
            err.response["Error"]["Message"],
        )
        raise
    return tables

def clear_tables(dyn_resource):
    try:
        for table in dyn_resource.tables.all():
            if table.name in ['retwis-users', 'tweets']:
                print("Deleting table:", table.name)
                table.delete()
    except ClientError as err:
        logger.error(
            "Couldn't delete tables. Here's why: %s: %s",
            err.response["Error"]["Code"],
            err.response["Error"]["Message"],
        )
        raise

if __name__ == "__main__":


    parser = argparse.ArgumentParser()
    parser.add_argument('-r', '--region', type=str, required=True)
    parser.add_argument('-s', '--source', type=str, required=True)
    parser.add_argument('-c', '--clear', action='store_true', default=False, help="Drop all the tables")

    args = parser.parse_args()

    dynamodb = boto3.resource('dynamodb', region_name=args.region)

    if args.clear:
        clear_tables(dynamodb)
        print("Exiting because the delete operation takes a bit of time.")
        exit(0)
    tables = list_tables(dynamodb)
    osm_table = OSM(dynamodb)

    all_tables = [osm_table]
    for table in all_tables:
        if table.table_name not in tables:
            print("Creating table:", table.table_name)
            table.create_table()
            print("Done creating table:", table.table_name)
        else:
            table.table = dynamodb.Table(table.table_name)


    osm_file_path = "map.osm"
    osm_data = parse_osm_file(osm_file_path)

    osm_table.insert_all_data(osm_data['nodes'], osm_data['ways'], osm_data['relations'])




