@echo off
echo ========================================
echo Starting BozoChat Client (Development)
echo ========================================
echo.

echo Installing dependencies...
call npm install

echo.
echo Starting client...
npm run dev
