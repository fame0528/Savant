#!/usr/bin/env node

const { spawn, exec } = require('child_process');
const path = require('path');
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

// Check if command exists
function commandExists(command) {
    return new Promise((resolve) => {
        const cmd = process.platform === 'win32' ? 'where' : 'which';
        exec(`${cmd} ${command}`, (error) => {
            resolve(!error);
        });
    });
}

// Create logs directory
if (!fs.existsSync('logs')) {
    fs.mkdirSync('logs');
}

// Store process PIDs
const processes = [];

// Cleanup function
function cleanup() {
    log('\n🛑 Shutting down Savant system...', 'yellow');
    
    processes.forEach(proc => {
        if (proc && !proc.killed) {
            proc.kill('SIGTERM');
        }
    });
    
    // Force kill any remaining processes
    if (process.platform === 'win32') {
        exec('taskkill /f /im cargo.exe /t >nul 2>&1');
        exec('taskkill /f /im node.exe /t >nul 2>&1');
    } else {
        exec('pkill -f "cargo run --bin savant_cli" 2>/dev/null');
        exec('pkill -f "npm run dev" 2>/dev/null');
    }
    
    log('✅ System shutdown complete', 'green');
    process.exit(0);
}

// Handle shutdown signals
process.on('SIGINT', cleanup);
process.on('SIGTERM', cleanup);

async function startSystem() {
    log('🚀 Starting Savant Complete System...', 'blue');
    
    // Check prerequisites
    log('🔍 Checking prerequisites...', 'blue');
    
    const hasCargo = await commandExists('cargo');
    const hasNode = await commandExists('node');
    const hasNpm = await commandExists('npm');
    
    if (!hasCargo) {
        log('❌ Rust/Cargo not found. Please install Rust first.', 'red');
        process.exit(1);
    }
    
    if (!hasNode) {
        log('❌ Node.js not found. Please install Node.js first.', 'red');
        process.exit(1);
    }
    
    if (!hasNpm) {
        log('❌ npm not found. Please install npm first.', 'red');
        process.exit(1);
    }
    
    log('✅ Prerequisites met', 'green');
    
    // Build the Rust project
    log('🔨 Building Savant core...', 'blue');
    await new Promise((resolve, reject) => {
        const build = spawn('cargo', ['build', '--release'], { stdio: 'inherit' });
        build.on('close', (code) => {
            if (code === 0) {
                log('✅ Core build complete', 'green');
                resolve();
            } else {
                log('❌ Build failed', 'red');
                reject(new Error('Build failed'));
            }
        });
    });
    
    // Install dashboard dependencies if needed
    if (!fs.existsSync('dashboard/node_modules')) {
        log('📦 Installing dashboard dependencies...', 'blue');
        await new Promise((resolve, reject) => {
            const npmInstall = spawn('npm', ['install'], {
                cwd: 'dashboard',
                stdio: 'inherit'
            });
            npmInstall.on('close', (code) => {
                if (code === 0) {
                    log('✅ Dependencies installed', 'green');
                    resolve();
                } else {
                    log('❌ Failed to install dependencies', 'red');
                    reject(new Error('npm install failed'));
                }
            });
        });
    }
    
    // Start the Gateway and Swarm
    log('🌐 Starting Savant Gateway and Swarm...', 'blue');
    const gateway = spawn('cargo', ['run', '--release', '--bin', 'savant_cli'], {
        stdio: ['ignore', fs.openSync('logs/gateway.log', 'w'), fs.openSync('logs/gateway.log', 'w')]
    });
    processes.push(gateway);
    log('✅ Savant Gateway started', 'green');
    
    // Wait for gateway to initialize
    await new Promise(resolve => setTimeout(resolve, 3000));
    
    // Start the Dashboard
    log('📊 Starting Savant Dashboard...', 'blue');
    const dashboard = spawn('npm', ['run', 'dev'], {
        cwd: 'dashboard',
        stdio: ['ignore', fs.openSync('logs/dashboard.log', 'w'), fs.openSync('logs/dashboard.log', 'w')]
    });
    processes.push(dashboard);
    log('✅ Savant Dashboard started', 'green');
    
    // Wait for dashboard to initialize
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    console.log('');
    log('🎉 Savant System is now running!', 'green');
    console.log('');
    log('📱 Savant Dashboard: http://localhost:3000', 'blue');
    log('🔗 Savant Gateway:  http://localhost:8080', 'blue');
    log('📋 Savant Logs:     ./logs/', 'blue');
    console.log('');
    log('Press Ctrl+C to stop all services', 'yellow');
    console.log('');
    
    // Show live logs
    log('📋 Live logs (Ctrl+C to stop):', 'blue');
    console.log('');
    
    // Tail logs
    const tail = spawn('tail', ['-f', 'logs/gateway.log', 'logs/dashboard.log'], { stdio: 'inherit' });
    processes.push(tail);
    
    // Wait for any process to exit
    gateway.on('close', (code) => {
        if (code !== 0) {
            log(`Gateway exited with code ${code}`, 'red');
        }
        cleanup();
    });
    
    dashboard.on('close', (code) => {
        if (code !== 0) {
            log(`Dashboard exited with code ${code}`, 'red');
        }
        cleanup();
    });
}

// Handle command line arguments
const isDev = process.argv.includes('--dev');

if (isDev) {
    log('🔧 Development Mode - Starting with file watching...', 'yellow');
}

startSystem().catch(error => {
    log(`❌ Failed to start system: ${error.message}`, 'red');
    process.exit(1);
});
