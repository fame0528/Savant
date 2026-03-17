@echo off
REM Savant Complete System Launcher for Windows
REM Starts Gateway, Dashboard, and all Swarm components

echo 🚀 Starting Savant Complete System...

REM Check prerequisites
echo 🔍 Checking prerequisites...

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ Rust/Cargo not found. Please install Rust first.
    pause
    exit /b 1
)

where node >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ Node.js not found. Please install Node.js first.
    pause
    exit /b 1
)

where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo ❌ npm not found. Please install npm first.
    pause
    exit /b 1
)

echo ✅ Prerequisites met

REM Clean up old processes to prevent target locks
echo 🛑 Cleaning up previous manifestation enclaves...
taskkill /f /im savant_cli.exe >nul 2>nul
taskkill /f /im cargo.exe >nul 2>nul

REM Parse command line arguments
set SKIP_BUILD=0
set FORCE_BUILD=1
if "%1"=="--fast" set SKIP_BUILD=1

if %SKIP_BUILD% equ 1 (
     echo ⚡ Fast startup enabled: Skipping build check...
    goto :start_services
)

REM Forced Build: Ensure latest diagnostics are Manifest
echo 🔨 Initializing core manifestation (Forced Build)...
goto :do_build

:do_build
echo 🔨 Building Savant core...
cargo build --release
if %errorlevel% neq 0 (
    echo ❌ Build failed
    pause
    exit /b 1
)
set RUN_CMD=cargo run --release --bin savant_cli
echo ✅ Core build complete

:start_services

REM Install dashboard dependencies if needed
if not exist "dashboard\node_modules" (
    echo 📦 Installing dashboard dependencies...
    cd dashboard
    npm install
    cd ..
    echo ✅ Dependencies installed
)

REM Create logs directory
if not exist "logs" mkdir logs

REM Start the Gateway and Swarm
echo 🌐 Starting Gateway and Swarm...
start "Savant Swarm Engine" cmd /k "%RUN_CMD%"
echo ✅ Gateway started

REM Wait a moment for gateway to initialize
timeout /t 3 /nobreak >nul

REM Start the Dashboard
echo 📊 Starting Dashboard...
cd dashboard
start "Savant Dashboard" cmd /k "npm run dev"
cd ..
echo ✅ Dashboard started

REM Wait a moment for dashboard to initialize
timeout /t 2 /nobreak >nul

echo.
echo 🎉 Savant System is now running!
echo.
echo 📱 Dashboard:     http://localhost:3000
echo 🔗 Gateway:      http://localhost:8080
echo 📋 Logs:         .\logs\
echo.
echo Press any key to stop all services...
pause >nul

REM Cleanup
echo 🛑 Shutting down Savant system...
taskkill /f /im cargo.exe >nul 2>nul
taskkill /f /im node.exe >nul 2>nul
echo ✅ System shutdown complete

pause
