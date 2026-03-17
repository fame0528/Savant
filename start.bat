@echo off
REM Savant Smart Launcher - builds only when source changes
REM Starts Gateway, Dashboard, and all Swarm components

echo Starting Savant Complete System...

REM Check prerequisites
echo Checking prerequisites...

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo Rust/Cargo not found. Please install Rust first.
    pause
    exit /b 1
)

where node >nul 2>nul
if %errorlevel% neq 0 (
    echo Node.js not found. Please install Node.js first.
    pause
    exit /b 1
)

where npm >nul 2>nul
if %errorlevel% neq 0 (
    echo npm not found. Please install npm first.
    pause
    exit /b 1
)

echo Prerequisites met

REM Clean up old processes to prevent target locks
echo Cleaning up previous processes...
taskkill /f /im savant_cli.exe >nul 2>nul
taskkill /f /im cargo.exe >nul 2>nul

REM Set the run command
set RUN_CMD=cargo run --release --bin savant_cli

REM Parse command line arguments
if "%1"=="--force" goto :do_build
if "%1"=="--skip" goto :start_services

REM Smart build: only rebuild if source files changed since last build
:smart_build
set NEED_BUILD=0

REM If no binary exists, must build
if not exist "target\release\savant_cli.exe" (
    echo No binary found. Building...
    set NEED_BUILD=1
    goto :do_build
)

REM Check if Cargo.toml is newer than binary
for %%F in ("target\release\savant_cli.exe") do set BIN_TIME=%%~tF
for %%F in ("Cargo.toml") do set CARGO_TIME=%%~tF

if "%CARGO_TIME%" gtr "%BIN_TIME%" (
    echo Cargo.toml changed since last build. Rebuilding...
    set NEED_BUILD=1
    goto :do_build
)

REM Check all .rs files in crates for changes
for /r crates %%F in (*.rs) do (
    if "%%~tF" gtr "%BIN_TIME%" (
        echo Source changed: %%~nxF
        set NEED_BUILD=1
        goto :do_build
    )
)

if %NEED_BUILD% equ 0 (
    echo Binary is up to date. Skipping build.
    goto :start_services
)

:do_build
echo Building Savant core...
cargo build --release
if %errorlevel% neq 0 (
    echo Build failed
    pause
    exit /b 1
)
echo Core build complete

:start_services

REM Install dashboard dependencies if needed
if not exist "dashboard\node_modules" (
    echo Installing dashboard dependencies...
    cd dashboard
    npm install
    cd ..
    echo Dependencies installed
)

REM Create required directories
if not exist "logs" mkdir logs
if not exist "data" mkdir data
if not exist "workspaces\substrate" mkdir workspaces\substrate
if not exist "workspaces\agents" mkdir workspaces\agents

REM Start the Gateway and Swarm
echo Starting Gateway and Swarm...
start "Savant Swarm Engine" cmd /k "%RUN_CMD%"

REM Wait for gateway to fully initialize - poll health endpoint
echo Waiting for gateway to be ready...
set GATEWAY_READY=0
set WAIT_COUNT=0
set MAX_WAIT=30

:wait_gateway
timeout /t 1 /nobreak >nul
set /a WAIT_COUNT=%WAIT_COUNT%+1

REM Try to hit the health endpoint
curl -s -o nul -w "%%{http_code}" http://localhost:3000/live 2>nul | findstr "200" >nul
if %errorlevel% equ 0 (
    set GATEWAY_READY=1
    echo Gateway is ready after %WAIT_COUNT% seconds
    goto :gateway_ready
)

if %WAIT_COUNT% geq %MAX_WAIT% (
    echo Gateway did not become ready within %MAX_WAIT% seconds
    echo Check the gateway window for errors
    goto :gateway_ready
)

goto :wait_gateway

:gateway_ready

REM Start the Dashboard
echo Starting Dashboard...
cd dashboard
start "Savant Dashboard" cmd /k "npm run dev"
cd ..
echo Dashboard started

REM Wait for dashboard to initialize
timeout /t 3 /nobreak >nul

echo.
echo Savant System is now running!
echo.
echo Dashboard:     http://localhost:3000
echo Gateway:       ws://localhost:3000
echo Logs:          .\logs\
echo.
echo Press any key to stop all services...
pause >nul

REM Cleanup
echo Shutting down Savant system...
taskkill /f /im cargo.exe >nul 2>nul
taskkill /f /im node.exe >nul 2>nul
echo System shutdown complete

pause
