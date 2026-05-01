@echo off
echo ========================================
echo  Savant Build Script
echo ========================================
echo.

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
del /F /Q "C:\Users\spenc\dev\Savant\dashboard\.next\lock" >nul 2>&1
if exist "C:\Users\spenc\dev\Savant\dashboard\.next\lock" (
    rmdir /S /Q "C:\Users\spenc\dev\Savant\dashboard\.next" >nul 2>&1
    echo   .next directory removed.
) else (
    echo   Lock file removed.
)

echo.
echo [3/4] Building dashboard...
cd /d "C:\Users\spenc\dev\Savant"
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
echo   MSI:  target\release\bundle\msi\Savant_0.1.1_x64_en-US.msi
echo   EXE:  target\release\bundle\nsis\Savant_0.1.1_x64-setup.exe
echo.
pause
