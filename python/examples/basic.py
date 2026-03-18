"""Basic Archflow example - Web Service architecture."""

from archflow import Cluster, Diagram, Node

with Diagram("Web Service", direction="LR") as d:
    with Cluster("vpc", "VPC"):
        web = Node("web", "Web Server")
        app = Node("app", "App Server")
        db = Node("db", "Database")

    web >> app >> db

    d.save_json("python/examples/basic.json")
    d.save_svg("python/examples/basic.svg")
