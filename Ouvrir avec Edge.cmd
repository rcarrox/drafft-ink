@echo off
setlocal EnableExtensions
cd /d "%~dp0"
if not exist "web\pkg\drafftink_app_bg.wasm" exit /b 1
if not exist ".drafftink-server.pid" (
  start "DrafftInk local server" /min powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0windows\serve-local.ps1" -Root "%~dp0web" -Port 8765
  timeout /t 2 /nobreak >nul
)
set "EDGE=%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe"
if not exist "%EDGE%" set "EDGE=%ProgramFiles%\Microsoft\Edge\Application\msedge.exe"
if exist "%EDGE%" (start "" "%EDGE%" "http://127.0.0.1:8765/") else (start "" "http://127.0.0.1:8765/")
