from diagrams import Diagram, Edge
from diagrams.aws.compute import EC2
from diagrams.programming.language import Rust
from diagrams.programming.framework import React
from diagrams.onprem.compute import Server

from diagrams.saas.chat import Discord

with Diagram("Discord Ticket Architecture"):
    discord = Discord("Serveurs Discords")
    discord_bot = Rust("Discord bot")
    backend = Rust("Backend")
    frontend = React("Webapp")
    client_server = Server("Client server")

    discord_bot >> Edge(label="Fetch Information and Create Channels/MC") >> discord
    backend >> Edge(label="Bidirectional talk") >> discord_bot
    frontend >> Edge(label="REST/GraphQL?") >> backend
    client_server >> Edge(label="REST/Webhook") >> backend
