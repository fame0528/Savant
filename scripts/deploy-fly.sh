#!/bin/bash
set -euo pipefail

if ! command -v flyctl &> /dev/null; then
    echo "Error: flyctl not found. Install from https://fly.io/docs/hands-on/install-flyctl/"
    exit 1
fi

echo "Launching Savant on Fly.io..."
flyctl launch --copy-config --yes

echo "Deploying..."
flyctl deploy

echo "Savant deployed to Fly.io"
echo "Check status: flyctl status"
echo "View logs:    flyctl logs"
