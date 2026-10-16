@echo off
rem Single player (you can still Host or Join from the Multiplayer button in the menu).
setlocal EnableExtensions
set "WOW_SURVIVORS_HOST="
set "WOW_SURVIVORS_JOIN="
@"%~dp0_launch.cmd" play %*
