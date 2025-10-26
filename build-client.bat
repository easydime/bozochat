@echo off
echo ========================================
echo Building BozoChat Client for Windows
echo ========================================
echo.

echo Installing dependencies...
call npm install

echo.
echo Building executable...
call npm run build:win

echo.
echo ========================================
echo Build complete!
echo.
echo The installer is located in: dist\
echo ========================================
pause
