#!/bin/bash
set -e

echo "🚀 Running local CI checks..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[CI CHECK]${NC} $1"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# 1. Format check
print_status "Running cargo fmt..."
if cargo fmt --all -- --check; then
    print_success "Formatting is correct"
else
    print_warning "Formatting issues detected - fixing automatically..."
    cargo fmt --all
    print_success "Code formatted successfully"
fi

# 2. Clippy check
print_status "Running cargo clippy..."
if cargo clippy --all-targets --workspace -- -D warnings; then
    print_success "No clippy warnings"
else
    print_error "Clippy warnings detected - please fix manually"
    exit 1
fi

# 3. Build check
print_status "Building workspace..."
if cargo build --workspace; then
    print_success "Build successful"
else
    print_error "Build failed"
    exit 1
fi

# 4. Test check
print_status "Running tests..."
if cargo test --workspace -- --nocapture; then
    print_success "All tests passed"
else
    print_error "Tests failed"
    exit 1
fi

print_success "🎉 All CI checks passed! Safe to push to remote."
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  git add -A"
echo "  git commit -m 'your commit message'"
echo "  git push"