# Raydium AMM Account Stream Service

This service streams and parses Raydium AMM account updates from the Solana blockchain using Shyft's gRPC API.

## Setup

1. **Copy the environment template:**
   ```bash
   cp env-template.txt .env
   ```

2. **Edit `.env` with your real Shyft API token:**
   ```bash
   # Edit the X_TOKEN line with your actual token
   X_TOKEN=your_real_shyft_token_here
   ```

## Running

### Option 1: Using the provided scripts
- **PowerShell:** `.\run-raydium-stream.ps1` (from project root)
- **Batch:** `run-raydium-stream.bat` (from project root)

### Option 2: Direct cargo run
```bash
cargo run
```
(This will automatically load variables from `.env`)

### Option 3: Command line arguments
```bash
cargo run -- --endpoint https://grpc.ny.shyft.to --x-token your_token
```

## Environment Variables

- `ENDPOINT`: Shyft gRPC endpoint (default: https://grpc.ny.shyft.to)
- `X_TOKEN`: Your Shyft API token (required)
- `RUST_LOG`: Log level (optional, default: info)

## Features

- ✅ Automatic .env file loading
- ✅ Environment variable support with defaults
- ✅ Parameter echoing on startup
- ✅ Raydium AMM account parsing
- ✅ Real-time account updates via gRPC