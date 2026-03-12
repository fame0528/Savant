#!/usr/bin/env node

const { exec } = require('child_process');
const fs = require('fs');
const chalk = require('chalk');

// ANSI color codes
const colors = {
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    blue: '\x1b[34m',
    reset: '\x1b[0m'
};

function log(message, color = 'reset') {
    console.log(`${colors[color]}${message}${colors.reset}`);
}

async function stopSystem() {
    log('🛑 Shutting down Savant system...', 'yellow');
    
    // Kill processes by PID if PID files exist
    if (fs.existsSync('logs/gateway.pid')) {
        const gatewayPid = fs.readFileSync('logs/gateway.pid', 'utf8').trim();
        try {
            process.kill(gatewayPid, 'SIGTERM');
            log('✅ Gateway stopped', 'green');
        } catch (error) {
            // Process might already be dead
        }
        fs.unlinkSync('logs/gateway.pid');
    }
    
    if (fs.existsSync('logs/dashboard.pid')) {
        const dashboardPid = fs.readFileSync('logs/dashboard.pid', 'utf8').trim();
        try {
            process.kill(dashboardPid, 'SIGTERM');
            log('✅ Dashboard stopped', 'green');
        } catch (error) {
            // Process might already be dead
        }
        fs.unlinkSync('logs/dashboard.pid');
    }
    
    // Force kill any remaining processes
    return new Promise((resolve) => {
        if (process.platform === 'win32') {
            exec('taskkill /f /im cargo.exe /t >nul 2>&1', () => {
                exec('taskkill /f /im node.exe /t >nul 2>&1', () => {
                    log('✅ All services stopped', 'green');
                    resolve();
                });
            });
        } else {
            exec('pkill -f "cargo run --bin savant_cli" 2>/dev/null', () => {
                exec('pkill -f "npm run dev" 2>/dev/null', () => {
                    log('✅ All services stopped', 'green');
                    resolve();
                });
            });
        }
    });
}

stopSystem().catch(error => {
    log(`❌ Failed to stop system: ${error.message}`, 'red');
    process.exit(1);
});
