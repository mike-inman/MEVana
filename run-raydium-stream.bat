@echo off
REM Batch file to run Raydium stream service
REM This will read from .env file in the crate directory

echo 🚀 Starting Raydium Stream Service (reading from .env file)...

cd crates\raydium\stream_and_parse_raydium_accounts

REM Check if .env file exists
if not exist ".env" (
    echo ❌ .env file not found!
    echo 📝 Copy env-template.txt to .env and edit with your real token:
    echo    copy env-template.txt .env
    pause
    exit /b 1
)

echo ✅ Found .env file, loading environment variables...
cargo run
