#!/bin/bash

# Check if pd-server container is running
if [ "$(sudo docker ps -q -f name=pd-server)" ]; then
    echo "pd-server is already running."
else
    # Check if pd-server container exists (stopped)
    if [ "$(sudo docker ps -aq -f status=exited -f name=pd-server)" ]; then
        # Start the existing container
        echo "Starting existing pd-server container..."
        sudo docker start pd-server
    else
        # Run a new pd-server container
        echo "Running new pd-server container..."
        sudo docker run -d --name pd-server --network host pingcap/pd:latest \
            --name="pd1" \
            --data-dir="/pd1" \
            --client-urls="http://0.0.0.0:2379" \
            --peer-urls="http://0.0.0.0:2380" \
            --advertise-client-urls="http://0.0.0.0:2379" \
            --advertise-peer-urls="http://0.0.0.0:2380"
    fi
fi
