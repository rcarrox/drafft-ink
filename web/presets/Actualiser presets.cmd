@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0actualiser.ps1"
if errorlevel 1 pause
