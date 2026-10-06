@echo off
setlocal EnableExtensions
cd /d "%~dp0"
if not exist "web\pkg\drafftink_app_bg.wasm" exit /b 1
if not exist ".drafftink-server.pid" (
  start "DrafftInk local server" /min powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0windows\serve-local.ps1" -Root "%~dp0web" -Port 8765
  timeout /t 2 /nobreak >nul
)
set "CHROME=%LocalAppData%\Google\Chrome\Application\chrome.exe"
if not exist "%CHROME%" set "CHROME=%ProgramFiles%\Google\Chrome\Application\chrome.exe"
if not exist "%CHROME%" if defined ProgramFiles(x86) set "CHROME=%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe"
if exist "%CHROME%" (start "" "%CHROME%" "http://127.0.0.1:8765/") else (start "" "http://127.0.0.1:8765/")
