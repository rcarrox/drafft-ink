@echo off
setlocal EnableExtensions
cd /d "%~dp0"

if not exist "web\pkg\drafftink_app_bg.wasm" (
  echo.
  echo DrafftInk n'est pas compile dans ce dossier.
  echo Telechargez l'archive Windows Portable produite par GitHub Actions.
  echo.
  pause
  exit /b 1
)

set "PORT=8765"
set "URL=http://127.0.0.1:%PORT%/"

if exist ".drafftink-server.pid" (
  set /p OLD_PID=<".drafftink-server.pid"
  tasklist /FI "PID eq %OLD_PID%" 2>nul | find "%OLD_PID%" >nul
  if errorlevel 1 del /q ".drafftink-server.pid" >nul 2>&1
)

if not exist ".drafftink-server.pid" (
  start "DrafftInk local server" /min powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0windows\serve-local.ps1" -Root "%~dp0web" -Port %PORT%
  timeout /t 2 /nobreak >nul
)

:menu
cls
echo ======================================
echo           Qraphtinc 0.17.0
echo ======================================
echo.
echo   1 - Google Chrome
echo   2 - Microsoft Edge
echo   3 - Navigateur par defaut
echo   4 - Quitter sans ouvrir
echo.
choice /C 1234 /N /M "Votre choix : "
if errorlevel 4 exit /b 0
if errorlevel 3 goto default
if errorlevel 2 goto edge
if errorlevel 1 goto chrome

goto menu

:chrome
set "BROWSER="
if exist "%LocalAppData%\Google\Chrome\Application\chrome.exe" set "BROWSER=%LocalAppData%\Google\Chrome\Application\chrome.exe"
if not defined BROWSER if exist "%ProgramFiles%\Google\Chrome\Application\chrome.exe" set "BROWSER=%ProgramFiles%\Google\Chrome\Application\chrome.exe"
if not defined BROWSER if defined ProgramFiles(x86) if exist "%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe" set "BROWSER=%ProgramFiles(x86)%\Google\Chrome\Application\chrome.exe"
if not defined BROWSER (
  echo Chrome est introuvable. Ouverture avec le navigateur par defaut.
  timeout /t 2 /nobreak >nul
  goto default
)
start "" "%BROWSER%" "%URL%"
exit /b 0

:edge
set "BROWSER="
if exist "%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe" set "BROWSER=%ProgramFiles(x86)%\Microsoft\Edge\Application\msedge.exe"
if not defined BROWSER if exist "%ProgramFiles%\Microsoft\Edge\Application\msedge.exe" set "BROWSER=%ProgramFiles%\Microsoft\Edge\Application\msedge.exe"
if not defined BROWSER (
  echo Edge est introuvable. Ouverture avec le navigateur par defaut.
  timeout /t 2 /nobreak >nul
  goto default
)
start "" "%BROWSER%" "%URL%"
exit /b 0

:default
start "" "%URL%"
exit /b 0
