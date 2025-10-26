@echo off
echo ========================================
echo Starting BozoChat Server
echo ========================================
echo.

echo Checking for .env file...
if not exist ".env" (
    echo ERROR: .env file not found!
    echo Please copy .env.example to .env and configure it
    pause
    exit /b 1
)

echo Installing dependencies...
call npm install

echo.
echo Starting bot server...
npm run bot
