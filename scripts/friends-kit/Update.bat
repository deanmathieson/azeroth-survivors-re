@echo off
rem Check for the latest official kit without starting the game.
setlocal EnableExtensions DisableDelayedExpansion
@"%~dp0_launch.cmd" --update-only %*
