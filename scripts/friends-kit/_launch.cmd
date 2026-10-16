@echo off
rem Shared by Play.bat, Host.bat and Join.bat. Finds the WoW 1.12.1 Data folder, keeps the
rem profile beside this kit, then starts the game. Not meant to be run on its own.
setlocal EnableExtensions DisableDelayedExpansion

set "KIT=%~dp0"
set "EXE=%KIT%warcraft-survivors.exe"
set "PATHFILE=%KIT%wow-data-path.txt"

rem Updating is deliberately best-effort: a party should still be able to play offline.
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -Command "& { . '%KIT%_update.ps1'; [void](Invoke-AzerothSurvivorsUpdate -KitPath '%KIT%') }"

if /i "%~1"=="--update-only" (
    endlocal
    exit /b 0
)

if not exist "%EXE%" (
    echo Cannot find warcraft-survivors.exe next to this file.
    echo Unzip the whole kit into one folder and run it from there.
    pause
    exit /b 1
)

rem Your profile, settings and saves live in benilla-config\ inside this folder.
set "BENILLA_HOME=%KIT%benilla-config"

set "TRIES=0"
set "WOW_DATA="
if exist "%PATHFILE%" set /p WOW_DATA=<"%PATHFILE%"

:check
if defined WOW_DATA set "WOW_DATA=%WOW_DATA:"=%"
if not defined WOW_DATA goto ask
if "%WOW_DATA:~-1%"=="\" set "WOW_DATA=%WOW_DATA:~0,-1%"
if exist "%WOW_DATA%\patch.MPQ" goto found
if exist "%WOW_DATA%\Data\patch.MPQ" (
    set "WOW_DATA=%WOW_DATA%\Data"
    goto found
)
echo.
echo No patch.MPQ found in:
echo     "%WOW_DATA%"
echo That is not a WoW 1.12.1 Data folder.

:ask
set /a TRIES+=1
if %TRIES% gtr 5 (
    echo No folder given. Bye for now.
    exit /b 1
)
echo.
echo Where is your WoW 1.12.1 "Data" folder? (the one full of .MPQ files)
echo Tip: shift + right-click the folder in Explorer, "Copy as path", then right-click here to paste.
set "WOW_DATA="
set /p "WOW_DATA=Data folder: "
if not defined WOW_DATA goto ask
goto check

:found
> "%PATHFILE%" echo(%WOW_DATA%
echo Using WoW data: "%WOW_DATA%"
if defined WOW_SURVIVORS_HOST echo Hosting on UDP port %WOW_SURVIVORS_HOST% as %WOW_SURVIVORS_NAME%
if defined WOW_SURVIVORS_JOIN echo Joining %WOW_SURVIVORS_JOIN% as %WOW_SURVIVORS_NAME%
echo Starting the game. Keep this window open; it shows the game's log.
echo.

"%EXE%"
if errorlevel 1 (
    echo.
    echo The game stopped with an error. Crash notes, if any, are in:
    echo     "%BENILLA_HOME%\Diagnostics"
    pause
)
endlocal
