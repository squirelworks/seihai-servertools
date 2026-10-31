@echo off
rem Seihai lab world: start the listserver and the gservers on 127.0.0.1 with
rem one command (Windows). Part of seihai-servertools (MIT OR Apache-2.0).
rem
rem Put this file in the lab world folder (next to gs2emu.exe and servers\),
rem with seihai-servertools.exe and seihai-listserver.exe beside it or on PATH.
rem
rem   start.bat            listserver, then servers\default and servers\classic
rem   start.bat classic    listserver, then only servers\classic
rem
rem Press Ctrl+C or close this window to stop them all. Consoles go to
rem servers\<name>\logs\console.txt and logs\listserver-console.txt.

setlocal
cd /d "%~dp0"

set TOOL=seihai-servertools.exe
if exist "%~dp0seihai-servertools.exe" set TOOL="%~dp0seihai-servertools.exe"
where seihai-servertools.exe >nul 2>nul
if errorlevel 1 if not exist "%~dp0seihai-servertools.exe" (
	echo start.bat: no seihai-servertools.exe next to this script or on PATH. Unpack a seihai-servertools release here.
	pause
	exit /b 1
)

%TOOL% lab start --world . %*
if errorlevel 1 pause
exit /b %errorlevel%
