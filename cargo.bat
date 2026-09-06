@echo off
set "PATH=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin;%USERPROFILE%\.cargo\bin;%PATH%"
"%USERPROFILE%\.cargo\bin\cargo.exe" %*

