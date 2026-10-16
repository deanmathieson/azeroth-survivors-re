@echo off
rem Join a friend's co-op game.
setlocal EnableExtensions
set "WOW_SURVIVORS_HOST="

set "WOW_SURVIVORS_NAME=%USERNAME%"
set /p "WOW_SURVIVORS_NAME=Your name in the party [%USERNAME%]: "
set "WOW_SURVIVORS_NAME=%WOW_SURVIVORS_NAME:"=%"

set "HOSTFILE=%~dp0last-host.txt"
set "WOW_SURVIVORS_JOIN="
if exist "%HOSTFILE%" set /p WOW_SURVIVORS_JOIN=<"%HOSTFILE%"

set "TRIES=0"
:ask
set /a TRIES+=1
if %TRIES% gtr 5 exit /b 1
if defined WOW_SURVIVORS_JOIN (
    set /p "WOW_SURVIVORS_JOIN=Host address, e.g. 100.64.0.7:5000 [%WOW_SURVIVORS_JOIN%]: "
) else (
    set /p "WOW_SURVIVORS_JOIN=Host address, e.g. 100.64.0.7:5000: "
)
if not defined WOW_SURVIVORS_JOIN goto ask
set "WOW_SURVIVORS_JOIN=%WOW_SURVIVORS_JOIN:"=%"
set "WOW_SURVIVORS_JOIN=%WOW_SURVIVORS_JOIN: =%"
> "%HOSTFILE%" echo(%WOW_SURVIVORS_JOIN%

@"%~dp0_launch.cmd" join %*
