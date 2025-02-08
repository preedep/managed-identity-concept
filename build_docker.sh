#!/bin/bash

# Set variables
IMAGE_NAME="my_api_server"
VERSION="latest"

echo "🐳 Building Docker image..."
docker buildx build --platform linux/amd64 -t $IMAGE_NAME:$VERSION .

echo "✅ Docker Image '$IMAGE_NAME:$VERSION' built successfully!"

# Optionally list built images
docker images | grep $IMAGE_NAME

# Run the container (Uncomment if needed)
# echo "🚀 Running Docker container..."
# docker run -d -p 8443:8443 --name my_rust_api_container $IMAGE_NAME:$VERSION
