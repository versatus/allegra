#!/bin/bash

# Check if tikv-server container is running
if [ "$(docker ps -q -f name=tikv-server)" ]; then
    echo "tikv-server is already running."
else
    # Check if tikv-server container exists (stopped)
    if [ "$(docker ps -aq -f status=exited -f name=tikv-server)" ]; then
        # Start the existing container
        echo "Starting existing tikv-server container..."
        docker start tikv-server
    else
        # Run a new tikv-server container
        echo "Running new tikv-server container..."
        docker run -d --name tikv-server --network host pingcap/tikv:latest \
            --addr="127.0.0.1:20160" \
            --advertise-addr="127.0.0.1:20160" \
            --data-dir="/tikv" \
            --pd="http://127.0.0.1:2379"
    fi
fi
