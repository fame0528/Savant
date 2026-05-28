@echo off
setlocal enabledelayedexpansion
echo ========================================
echo  Savant Build Script
echo ========================================
echo.

:: Resolve repo root from script location
set "REPO_ROOT=%~dp0"
if "%REPO_ROOT:~-1%"=="\" set "REPO_ROOT=%REPO_ROOT:~0,-1%"

:: Parse argument (default: both)
set "BUILD_TARGET=%1"
if "%BUILD_TARGET%"=="" set "BUILD_TARGET=both"

echo Build target: %BUILD_TARGET%
echo.

:: ?? Step 1: Clean stale processes ??????????????????????????????????
echo [1/4] Cleaning stale node processes...
taskkill /F /IM node.exe /FI "WINDOWTITLE eq savant*" >nul 2>&1
:: Only kill node if .next lock exists (avoids killing unrelated node apps)
if exist "%REPO_ROOT%\dashboard\.next\lock" (
    taskkill /F /IM node.exe >nul 2>&1
    timeout /t 2 /nobreak >nul
    echo   Stale node processes cleaned.
) else (
    echo   No stale lock found, skipping.
)

:: ?? Step 2: Clean Next.js build artifacts ??????????????????????????
echo.
echo [2/4] Cleaning Next.js build artifacts...
if exist "%REPO_ROOT%\dashboard\.next\lock" (
    del /F /Q "%REPO_ROOT%\dashboard\.next\lock" >nul 2>&1
)
if exist "%REPO_ROOT%\dashboard\.next" (
    rmdir /S /Q "%REPO_ROOT%\dashboard\.next" >nul 2>&1
    echo   .next directory removed.
) else (
    echo   Clean.
)

:: ?? Step 3: Validate dashboard (Next.js) ???????????????????????????
echo.
echo [3/4] Validating dashboard build...
cd /d "%REPO_ROOT%" || (echo ERROR: Cannot reach repo root! & pause & exit /b 1)
call npm --prefix dashboard run build
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Dashboard build failed!
    pause
    exit /b 1
)
echo   Dashboard validation passed.

:: ?? Step 4: Build Tauri installers ?????????????????????????????????
:: NOTE: cargo tauri build defaults to release mode (handles Rust compilation + bundling).
:: Use --debug flag explicitly if debug build is needed.
echo.
echo [4/4] Building Tauri installers (release mode)...

if /i "%BUILD_TARGET%"=="desktop" goto :build_desktop
if /i "%BUILD_TARGET%"=="cli" goto :build_cli
goto :build_both

:build_both
call :do_build "Savant Desktop" "%REPO_ROOT%\crates\desktop\src-tauri"
if %errorlevel% neq 0 exit /b 1
echo.
call :do_build "CLI Companion" "%REPO_ROOT%\crates\cli\crates\gui\src-tauri"
if %errorlevel% neq 0 exit /b 1
goto :done

:build_desktop
call :do_build "Savant Desktop" "%REPO_ROOT%\crates\desktop\src-tauri"
if %errorlevel% neq 0 exit /b 1
goto :done

:build_cli
call :do_build "CLI Companion" "%REPO_ROOT%\crates\cli\crates\gui\src-tauri"
if %errorlevel% neq 0 exit /b 1
goto :done

:: ?? Subroutine: build + retry ??????????????????????????????????????
:do_build
set "BUILD_NAME=%~1"
set "BUILD_DIR=%~2"
echo   Building %BUILD_NAME%...
cd /d "%BUILD_DIR%" || (echo ERROR: Cannot reach %BUILD_DIR%! & exit /b 1)
call cargo tauri build --bundles msi,nsis
if %errorlevel% neq 0 (
    echo   %BUILD_NAME% failed, retrying after 15s cooldown...
    timeout /t 15 /nobreak >nul
    call cargo tauri build --bundles msi,nsis
    if %errorlevel% neq 0 (
        echo ERROR: %BUILD_NAME% failed after retry!
        cd /d "%REPO_ROOT%"
        exit /b 1
    )
)
cd /d "%REPO_ROOT%"
echo   %BUILD_NAME% complete.
exit /b 0

:done
echo.
echo ========================================
echo  BUILD COMPLETE
echo ========================================
echo.
echo Installers are in:  target\release\bundle\
echo   Desktop MSI:  msi\Savant_*_x64_en-US.msi
echo   Desktop EXE:  nsis\Savant_*_x64-setup.exe
echo   CLI MSI:      msi\Savant CLI Companion_*_x64_en-US.msi
echo   CLI EXE:      nsis\Savant CLI Companion_*_x64-setup.exe
echo.
echo Usage: build.bat [desktop^|cli^|both]
echo.
pause
