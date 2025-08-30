# PowerShell script to run Raydium stream service
# This will read from .env file in the crate directory

Write-Host "🚀 Starting Raydium Stream Service (reading from .env file)..."

Set-Location "crates/raydium/stream_and_parse_raydium_accounts"

# Check if .env file exists
if (!(Test-Path ".env")) {
    Write-Host "❌ .env file not found!"
    Write-Host "📝 Copy env-template.txt to .env and edit with your real token:"
    Write-Host "   cp env-template.txt .env"
    exit 1
}

Write-Host "✅ Found .env file, loading environment variables..."
& cargo run
