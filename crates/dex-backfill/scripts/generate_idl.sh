#!/bin/bash

# AI-Driven IDL Generation Utility
# This script generates fresh IDL files from live Solana programs
# Useful for keeping type definitions up-to-date and for AI analysis

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
CLUSTER="mainnet"
OUTPUT_DIR="idls"
PROGRAM_ID=""

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 -p <PROGRAM_ID> [-c <CLUSTER>] [-o <OUTPUT_DIR>]"
    echo ""
    echo "Options:"
    echo "  -p <PROGRAM_ID>    Solana program ID (required)"
    echo "  -c <CLUSTER>       Solana cluster (default: mainnet)"
    echo "  -o <OUTPUT_DIR>    Output directory (default: idls)"
    echo "  -h                 Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 -p whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
    echo "  $0 -p whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc -c devnet"
    echo "  $0 -p whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc -o custom_idls"
}

# Parse command line arguments
while getopts "p:c:o:h" opt; do
    case $opt in
        p) PROGRAM_ID="$OPTARG" ;;
        c) CLUSTER="$OPTARG" ;;
        o) OUTPUT_DIR="$OPT_DIR" ;;
        h) show_usage; exit 0 ;;
        \?) print_error "Invalid option: -$OPTARG"; show_usage; exit 1 ;;
    esac
done

# Check if program ID is provided
if [ -z "$PROGRAM_ID" ]; then
    print_error "Program ID is required"
    show_usage
    exit 1
fi

# Check if Anchor is installed
if ! command -v anchor &> /dev/null; then
    print_error "Anchor CLI is not installed. Please install it first:"
    echo "  cargo install --git https://github.com/coral-xyz/anchor avm --locked --force"
    exit 1
fi

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Generate filename based on program ID (first 8 chars)
PROGRAM_SHORT=$(echo "$PROGRAM_ID" | cut -c1-8)
OUTPUT_FILE="$OUTPUT_DIR/${PROGRAM_SHORT}_idl.json"
BACKUP_FILE="$OUTPUT_DIR/${PROGRAM_SHORT}_idl_backup_$(date +%Y%m%d_%H%M%S).json"

print_status "Generating IDL for program: $PROGRAM_ID"
print_status "Cluster: $CLUSTER"
print_status "Output: $OUTPUT_FILE"

# Backup existing file if it exists
if [ -f "$OUTPUT_FILE" ]; then
    print_status "Backing up existing IDL to: $BACKUP_FILE"
    cp "$OUTPUT_FILE" "$BACKUP_FILE"
fi

# Generate fresh IDL using Anchor
print_status "Fetching IDL from live program..."
if anchor idl fetch "$PROGRAM_ID" --provider.cluster "$CLUSTER" -o "$OUTPUT_FILE"; then
    print_success "IDL generated successfully!"
    
    # Show file info
    FILE_SIZE=$(wc -c < "$OUTPUT_FILE")
    LINE_COUNT=$(wc -l < "$OUTPUT_FILE")
    print_status "File size: $FILE_SIZE bytes"
    print_status "Line count: $LINE_COUNT"
    
    # Validate JSON
    if jq empty "$OUTPUT_FILE" 2>/dev/null; then
        print_success "IDL is valid JSON"
    else
        print_warning "IDL may not be valid JSON - please verify"
    fi
    
    # Show first few lines for verification
    print_status "First few lines of generated IDL:"
    head -10 "$OUTPUT_FILE"
    
else
    print_error "Failed to generate IDL"
    exit 1
fi

print_success "IDL generation complete! File saved to: $OUTPUT_FILE"
print_status "You can now use this IDL in your Rust code for up-to-date type definitions."
