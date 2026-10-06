@echo off
setlocal EnableExtensions
cd /d "%~dp0"
if not exist ".drafftink-server.pid" (
  echo Aucun serveur DrafftInk local n'est en cours.
  timeout /t 2 /nobreak >nul
  exit /b 0
)
set /p PID_TO_STOP=<".drafftink-server.pid"
if not defined PID_TO_STOP exit /b 0
taskkill /PID %PID_TO_STOP% /T /F >nul 2>&1
del /q ".drafftink-server.pid" >nul 2>&1
exit /b 0
