@echo off
echo ========================================
echo  Savant Build Script
echo ========================================
echo.

:: Resolve repo root from script location
set "REPO_ROOT=%~dp0"
:: Remove trailing backslash
if "%REPO_ROOT:~-1%"=="\" set "REPO_ROOT=%REPO_ROOT:~0,-1%"

:: Parse argument (default: both)
set "BUILD_TARGET=%1"
if "%BUILD_TARGET%"=="" set "BUILD_TARGET=both"

echo Build target: %BUILD_TARGET%
echo.

echo [1/5] Killing node processes...
taskkill /F /IM node.exe >nul 2>&1
if %errorlevel% equ 0 (
    echo   Node processes killed.
) else (
    echo   No node processes found.
)
timeout /t 2 /nobreak >nul

echo.
echo [2/5] Removing Next.js lock file...
del /F /Q "%REPO_ROOT%\dashboard\.next\lock" >nul 2>&1
if exist "%REPO_ROOT%\dashboard\.next\lock" (
    rmdir /S /Q "%REPO_ROOT%\dashboard\.next" >nul 2>&1
    echo   .next directory removed.
) else (
    echo   Lock file removed.
)

echo.
echo [3/5] Building dashboard...
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
echo [4/5] Building Rust release binary...
call cargo build --release
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Rust compilation failed!
    pause
    exit /b 1
)
echo   Compilation complete.

echo.
echo [5/5] Building Tauri installers...
echo   Waiting for antivirus scan to complete...
timeout /t 10 /nobreak >nul

if /i "%BUILD_TARGET%"=="desktop" (
    goto :build_desktop
)
if /i "%BUILD_TARGET%"=="cli" (
    goto :build_cli
)

:build_both
echo   Building Savant Desktop...
cd /d "%REPO_ROOT%\crates\desktop\src-tauri"
call cargo tauri build --bundles msi,nsis
if %errorlevel% neq 0 (
    echo   Desktop build failed, retrying after delay...
    timeout /t 15 /nobreak >nul
    call cargo tauri build --bundles msi,nsis
)
cd /d "%REPO_ROOT%"
echo.
echo   Building CLI Companion...
cd /d "%REPO_ROOT%\crates\cli\crates\gui\src-tauri"
call cargo tauri build --bundles msi,nsis
if %errorlevel% neq 0 (
    echo   CLI build failed, retrying after delay...
    timeout /t 15 /nobreak >nul
    call cargo tauri build --bundles msi,nsis
)
cd /d "%REPO_ROOT%"
goto :done

:build_desktop
echo   Building Savant Desktop...
cd /d "%REPO_ROOT%\crates\desktop\src-tauri"
call cargo tauri build --bundles msi,nsis
if %errorlevel% neq 0 (
    echo   Desktop build failed, retrying after delay...
    timeout /t 15 /nobreak >nul
    call cargo tauri build --bundles msi,nsis
    if %errorlevel% neq 0 (
        echo ERROR: Desktop build failed after retry!
        cd /d "%REPO_ROOT%"
        pause
        exit /b 1
    )
)
cd /d "%REPO_ROOT%"
goto :done

:build_cli
echo   Building CLI Companion...
cd /d "%REPO_ROOT%\crates\cli\crates\gui\src-tauri"
call cargo tauri build --bundles msi,nsis
if %errorlevel% neq 0 (
    echo   CLI build failed, retrying after delay...
    timeout /t 15 /nobreak >nul
    call cargo tauri build --bundles msi,nsis
    if %errorlevel% neq 0 (
        echo ERROR: CLI build failed after retry!
        cd /d "%REPO_ROOT%"
        pause
        exit /b 1
    )
)
cd /d "%REPO_ROOT%"
goto :done

:done
echo.
echo ========================================
echo  BUILD COMPLETE
echo ========================================
echo.
echo Installers are located at:
echo   Desktop MSI:  target\release\bundle\msi\Savant_*_x64_en-US.msi
echo   Desktop EXE:  target\release\bundle\nsis\Savant_*_x64-setup.exe
echo   CLI MSI:      target\release\bundle\msi\Savant CLI Companion_*_x64_en-US.msi
echo   CLI EXE:      target\release\bundle\nsis\Savant CLI Companion_*_x64-setup.exe
echo.
echo Usage: build.bat [desktop^|cli^|both]
echo.
pause
