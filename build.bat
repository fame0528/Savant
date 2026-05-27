@echo off
echo ========================================
echo  Savant Build Script
echo ========================================
echo.

:: Resolve repo root from script location
set "REPO_ROOT=%~dp0"
:: Remove trailing backslash
if "%REPO_ROOT:~-1%"=="\" set "REPO_ROOT=%REPO_ROOT:~0,-1%"

echo [1/4] Killing node processes...
taskkill /F /IM node.exe >nul 2>&1
if %errorlevel% equ 0 (
    echo   Node processes killed.
) else (
    echo   No node processes found.
)
timeout /t 2 /nobreak >nul

echo.
echo [2/4] Removing Next.js lock file...
del /F /Q "%REPO_ROOT%\dashboard\.next\lock" >nul 2>&1
if exist "%REPO_ROOT%\dashboard\.next\lock" (
    rmdir /S /Q "%REPO_ROOT%\dashboard\.next" >nul 2>&1
    echo   .next directory removed.
) else (
    echo   Lock file removed.
)

echo.
echo [3/4] Building dashboard...
cd /d "%REPO_ROOT%"
call npm --prefix dashboard run build
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Dashboard build failed!
    pause
    exit /b 1
)
echo   Dashboard build complete.

echo.
echo [4/4] Building Tauri installer...
echo   Compiling release binary...
call cargo build --release
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Rust compilation failed!
    pause
    exit /b 1
)
echo   Compilation complete.

echo   Waiting for antivirus scan to complete...
timeout /t 10 /nobreak >nul

echo   Running bundler...
call cargo tauri build --bundles msi,nsis
if %errorlevel% neq 0 (
    echo.
    echo   Bundler failed, retrying after delay...
    timeout /t 15 /nobreak >nul
    call cargo tauri build --bundles msi,nsis
    if %errorlevel% neq 0 (
        echo.
        echo ERROR: Tauri build failed after retry!
        echo   Try running as Administrator or excluding the target folder from antivirus.
        pause
        exit /b 1
    )
)

echo.
echo ========================================
echo  BUILD COMPLETE
echo ========================================
echo.
echo Installers are located at:
echo   MSI:  target\release\bundle\msi\Savant CLI Companion_0.3.2_x64_en-US.msi
echo   EXE:  target\release\bundle\nsis\Savant CLI Companion_0.3.2_x64-setup.exe
echo.
pause
