#!/bin/bash
set -euo pipefail

echo "Building Savant Docker image..."
docker compose build

echo "Starting Savant gateway + Ollama..."
docker compose up -d

echo "Savant running at http://localhost:8080"
echo "Ollama running at http://localhost:11434"
echo ""
echo "Check status:  curl http://localhost:8080/ready"
echo "View logs:     docker compose logs -f gateway"
