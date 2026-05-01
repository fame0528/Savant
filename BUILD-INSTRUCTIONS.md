# Build Instructions - Continuous Awareness Architecture

## Step 1: Kill Node Processes
```
taskkill /F /IM node.exe
```

## Step 2: Remove Lock File
```
del /F /Q C:\Users\spenc\dev\Savant\dashboard\.next\lock
```

## Step 3: Build Dashboard
```
cd C:\Users\spenc\dev\Savant
npm --prefix dashboard run build
```

## Step 4: Build Tauri Installer
```
cargo tauri build
```

## Step 5: Install
Install the new build:
```
C:\Users\spenc\dev\Savant\target\release\bundle\nsis\Savant_0.1.1_x64-setup.exe
```

## Changes Included
1. **Copy All button** - logs.html now has working copy function
2. **Deterministic CPU metrics** - sysinfo crate replaces PowerShell, agent cannot hallucinate
3. **Unguided reflections** - Stillness reflections now use blank prompt, no environment steering
4. **Path resolution fix** - secure_resolve_path now allows project-root absolute paths
5. **safe_max_tokens fix** - Removed artificial cap, returns 85% of context_length
