@echo off
rem Host a co-op game. Friends run Join.bat with your Tailscale IP.
setlocal EnableExtensions
set "WOW_SURVIVORS_JOIN="

set "WOW_SURVIVORS_NAME=%USERNAME%"
set /p "WOW_SURVIVORS_NAME=Your name in the party [%USERNAME%]: "
set "WOW_SURVIVORS_NAME=%WOW_SURVIVORS_NAME:"=%"

set "WOW_SURVIVORS_HOST=5000"
set /p "WOW_SURVIVORS_HOST=UDP port to host on [5000]: "
set "WOW_SURVIVORS_HOST=%WOW_SURVIVORS_HOST:"=%"

echo.
echo Friends join you at your Tailscale IP, for example 100.x.y.z:%WOW_SURVIVORS_HOST%
echo (open Tailscale from the tray and click your PC's IP to copy it).
echo If Windows asks about the firewall, tick "Private networks" and Allow.
@"%~dp0_launch.cmd" host %*
