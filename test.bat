@echo off
echo ========================================
echo  Savant Test Runner (CPU-Limited)
echo ========================================
echo.

set "REPO_ROOT=%~dp0"
if "%REPO_ROOT:~-1%"=="\" set "REPO_ROOT=%REPO_ROOT:~0,-1%"

set "CARGO_BUILD_JOBS=2"

echo Running workspace tests with --test-threads=2...
echo.

cd /d "%REPO_ROOT%"
cargo test --workspace -- --test-threads=2
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Tests failed!
    pause
    exit /b 1
)

echo.
echo ========================================
echo  ALL TESTS PASSED
echo ========================================
echo.
pause
