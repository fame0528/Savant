@echo off
REM Savant Dev Mode - Fast iteration, no Tauri build
REM Starts Gateway + Dashboard separately for quick dev cycles

echo === SAVANT DEV MODE ===
echo.

REM Kill previous instances
echo Cleaning up...
taskkill /f /im savant_cli.exe >nul 2>nul
taskkill /f /im cargo.exe >nul 2>nul
timeout /t 1 /nobreak >nul

REM Create required directories
if not exist "logs" mkdir logs
if not exist "data" mkdir data
if not exist "workspaces\substrate" mkdir workspaces\substrate
if not exist "workspaces\agents" mkdir workspaces\agents

REM Start Gateway
echo Starting Gateway on port 8080...
start "Savant Gateway" cmd /k "cargo run --release --bin savant_cli"

REM Wait for gateway
echo Waiting for gateway...
set GATEWAY_READY=0
set WAIT_COUNT=0

:wait_loop
timeout /t 1 /nobreak >nul
set /a WAIT_COUNT=%WAIT_COUNT%+1
curl -s -o nul -w "%%{http_code}" http://localhost:8080/live 2>nul | findstr "200" >nul
if %errorlevel% equ 0 (
    echo Gateway ready after %WAIT_COUNT% seconds
    goto :start_dashboard
)
if %WAIT_COUNT% geq 30 (
    echo Gateway timeout - check gateway window
    goto :start_dashboard
)
goto :wait_loop

:start_dashboard
echo Starting Dashboard (dev mode)...
cd dashboard
start "Savant Dashboard" cmd /k "npm run dev"
cd ..

echo.
echo === DEV ENVIRONMENT READY ===
echo Gateway:  http://localhost:8080
echo Dashboard: http://localhost:3000
echo.
echo Close the terminal windows to stop.
echo Press any key to stop all services...
pause >nul

echo Shutting down...
taskkill /f /im cargo.exe >nul 2>nul
taskkill /f /im node.exe >nul 2>nul
echo Done.
