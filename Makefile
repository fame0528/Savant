# Savant System Makefile
# Provides convenient commands for development and deployment

.PHONY: help build start stop clean logs dev test

# Default target
help:
	@echo "Savant System Commands:"
	@echo ""
	@echo "  make build     - Build all Savant components"
	@echo "  make start     - Start complete Savant system (gateway + dashboard)"
	@echo "  make gateway   - Start only Savant gateway and swarm"
	@echo "  make dashboard - Start only Savant dashboard"
	@echo "  make stop      - Stop all running Savant services"
	@echo "  make logs      - Show live Savant logs"
	@echo "  make dev       - Development mode (start with file watching)"
	@echo "  make test      - Run all Savant tests"
	@echo "  make clean     - Clean Savant build artifacts and logs"
	@echo ""
	@echo "Quick start: make start"

# Build all components
build:
	@echo "🔨 Building Savant core..."
	cargo build --release
	@echo "📦 Installing dashboard dependencies..."
	cd dashboard && npm install && cd ..
	@echo "✅ Build complete"

# Start complete system
start:
	@echo "🚀 Starting Savant Complete System..."
	@if [ ! -d "logs" ]; then mkdir logs; fi
	@echo "🌐 Starting Savant Gateway and Swarm..."
	@cargo run --release --bin savant_cli > logs/gateway.log 2>&1 & echo $$! > logs/gateway.pid
	@sleep 3
	@echo "📊 Starting Savant Dashboard..."
	@cd dashboard && npm run dev > ../logs/dashboard.log 2>&1 & echo $$! > ../logs/dashboard.pid && cd ..
	@sleep 2
	@echo "🎉 Savant System is running!"
	@echo "📱 Savant Dashboard: http://localhost:3000"
	@echo "🔗 Savant Gateway:  http://localhost:8080"
	@echo "📋 Savant Logs:     ./logs/"

# Start only gateway
gateway:
	@echo "🌐 Starting Savant Gateway and Swarm..."
	@if [ ! -d "logs" ]; then mkdir logs; fi
	@cargo run --release --bin savant_cli

# Start only dashboard
dashboard:
	@echo "📊 Starting Savant Dashboard..."
	@cd dashboard && npm run dev

# Stop all services
stop:
	@echo "🛑 Stopping Savant services..."
	@if [ -f logs/gateway.pid ]; then kill $$(cat logs/gateway.pid) 2>/dev/null || true; rm logs/gateway.pid; fi
	@if [ -f logs/dashboard.pid ]; then kill $$(cat logs/dashboard.pid) 2>/dev/null || true; rm logs/dashboard.pid; fi
	@pkill -f "cargo run --bin savant_cli" 2>/dev/null || true
	@pkill -f "npm run dev" 2>/dev/null || true
	@echo "✅ Services stopped"

# Show live logs
logs:
	@echo "📋 Live logs (Ctrl+C to exit):"
	@tail -f logs/gateway.log logs/dashboard.log 2>/dev/null || echo "No logs available yet"

# Development mode with file watching
dev:
	@echo "🔧 Development Mode - Starting with file watching..."
	@if [ ! -d "logs" ]; then mkdir logs; fi
	@echo "🌐 Starting Gateway (with auto-restart)..."
	@while true; do cargo run --release --bin savant_cli > logs/gateway.log 2>&1; echo "Gateway crashed, restarting in 5 seconds..."; sleep 5; done &
	@sleep 3
	@echo "📊 Starting Dashboard..."
	@cd dashboard && npm run dev > ../logs/dashboard.log 2>&1 &
	@echo "🔧 Development mode active"
	@echo "📋 Logs: ./logs/"

# Run tests
test:
	@echo "🧪 Running tests..."
	cargo test
	cd dashboard && npm test && cd ..
	@echo "✅ Tests complete"

# Clean build artifacts and logs
clean:
	@echo "🧹 Cleaning up..."
	cargo clean
	cd dashboard && rm -rf node_modules .next && cd ..
	rm -rf logs
	rm -f savant.db
	@echo "✅ Clean complete"
