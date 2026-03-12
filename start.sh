#!/bin/bash

# Savant Complete System Launcher
# Starts Gateway, Dashboard, and all Swarm components

set -e

echo "🚀 Starting Savant Complete System..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check prerequisites
echo -e "${BLUE}🔍 Checking prerequisites...${NC}"

if ! command_exists cargo; then
    echo -e "${RED}❌ Rust/Cargo not found. Please install Rust first.${NC}"
    exit 1
fi

if ! command_exists node; then
    echo -e "${RED}❌ Node.js not found. Please install Node.js first.${NC}"
    exit 1
fi

if ! command_exists npm; then
    echo -e "${RED}❌ npm not found. Please install npm first.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Prerequisites met${NC}"

# Build the Rust project
echo -e "${BLUE}🔨 Building Savant core...${NC}"
cargo build --release
echo -e "${GREEN}✅ Core build complete${NC}"

# Install dashboard dependencies if needed
if [ ! -d "dashboard/node_modules" ]; then
    echo -e "${BLUE}📦 Installing dashboard dependencies...${NC}"
    cd dashboard
    npm install
    cd ..
    echo -e "${GREEN}✅ Dependencies installed${NC}"
fi

# Create logs directory
mkdir -p logs

# Function to cleanup background processes
cleanup() {
    echo -e "${YELLOW}🛑 Shutting down Savant system...${NC}"
    
    # Kill background processes
    if [ ! -z "$GATEWAY_PID" ]; then
        kill $GATEWAY_PID 2>/dev/null || true
    fi
    
    if [ ! -z "$DASHBOARD_PID" ]; then
        kill $DASHBOARD_PID 2>/dev/null || true
    fi
    
    echo -e "${GREEN}✅ System shutdown complete${NC}"
    exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM

# Start the Gateway and Swarm
echo -e "${BLUE}🌐 Starting Gateway and Swarm...${NC}"
cargo run --release --bin savant_cli > logs/gateway.log 2>&1 &
GATEWAY_PID=$!
echo -e "${GREEN}✅ Gateway started (PID: $GATEWAY_PID)${NC}"

# Wait a moment for gateway to initialize
sleep 3

# Start the Dashboard
echo -e "${BLUE}📊 Starting Dashboard...${NC}"
cd dashboard
npm run dev > ../logs/dashboard.log 2>&1 &
DASHBOARD_PID=$!
cd ..
echo -e "${GREEN}✅ Dashboard started (PID: $DASHBOARD_PID)${NC}"

# Wait a moment for dashboard to initialize
sleep 2

echo ""
echo -e "${GREEN}🎉 Savant System is now running!${NC}"
echo ""
echo -e "${BLUE}📱 Dashboard:${NC}     http://localhost:3000"
echo -e "${BLUE}🔗 Gateway:${NC}      http://localhost:8080"
echo -e "${BLUE}📋 Logs:${NC}         ./logs/"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"
echo ""

# Show live logs
echo -e "${BLUE}📋 Live logs (Ctrl+C to stop):${NC}"
echo ""

# Tail logs in background and wait for processes
tail -f logs/gateway.log logs/dashboard.log &
TAIL_PID=$!

# Wait for any background process to exit
wait $GATEWAY_PID $DASHBOARD_PID

# Cleanup
cleanup
