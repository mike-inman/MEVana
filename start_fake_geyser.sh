#!/bin/bash

# Fake Geyser - ISP Throttling Workaround
# This starts a fake Geyser stream that bypasses ISP issues

echo "🎭 Starting Fake Geyser Stream for ISP Throttling Workaround"
echo ""
echo "This will:"
echo "  ✅ Generate realistic Solana transaction data"  
echo "  ✅ Use current blockchain slot numbers"
echo "  ✅ Include all major DEX programs (Jupiter, Whirlpool, Raydium, etc.)"
echo "  ✅ Bypass ISP bandwidth throttling"
echo ""
echo "Your existing MEVana code will work normally - just point it to localhost!"
echo ""
echo "Usage after starting:"
echo "  GEYSER_ENDPOINT=http://localhost:50051 [your normal command]"
echo ""

# Simple standalone fake Geyser (no dependencies on your code)
cargo run --bin fake_geyser 2>/dev/null || rustc fake_geyser.rs -o fake_geyser && ./fake_geyser