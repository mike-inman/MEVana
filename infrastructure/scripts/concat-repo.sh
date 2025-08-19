#!/usr/bin/env bash

set -euo pipefail

USER=$1
REPO=$2
BRANCH=$3
FORMAT=$4 # text | markdown | html
shift 4
EXCLUDES=("${@:-}")

OUTPUT="combined_source.${FORMAT}"
DOCS_DIR="docs"

API_URL="https://api.github.com/repos/$USER/$REPO/git/trees/$BRANCH?recursive=1"
RAW_URL="https://raw.githubusercontent.com/$USER/$REPO/$BRANCH"

echo "Fetching file list from $USER/$REPO ($BRANCH)..."
FILES=$(curl -s "$API_URL" | jq -r '.tree[] | select(.type=="blob") | .path')

echo "DEBUG: Raw file count: $(echo "$FILES" | wc -l)"

EXCLUDE_REGEX=""
if [[ ${#EXCLUDES[@]} -gt 0 ]]; then
    EXCLUDE_REGEX=$(printf "|%s" "${EXCLUDES[@]}")
    EXCLUDE_REGEX=${EXCLUDE_REGEX:1}
    echo "DEBUG: Exclude regex: $EXCLUDE_REGEX"
fi

echo "Generating $OUTPUT..."
echo "" > "$OUTPUT"

TOC_TEMP=$(mktemp)

# Markdown header
if [[ "$FORMAT" == "markdown" ]]; then
    echo "# Combined Repo Source" >> "$OUTPUT"
    echo "" >> "$OUTPUT"
    echo "## Table of Contents" >> "$OUTPUT"
    echo "(TOC will be inserted here)" >> "$OUTPUT"
fi

# Add header function
add_header() {
    local file_path="$1"
    if [[ "$FORMAT" == "markdown" ]]; then
        local anchor
        anchor=$(echo "$file_path" | tr -d '/' | tr -d '.' | tr '[:upper:]' '[:lower:]')
        echo "- [$file_path](#${anchor})" >> "$TOC_TEMP"
        echo -e "\n## File: \`$file_path\`\n" >> "$OUTPUT"
    else
        echo -e "\n===== FILE: $file_path =====\n" >> "$OUTPUT"
    fi
}

# Add file content with line numbers
add_file_content() {
    local url="$1"
    local line_number=1
    curl -s "$url" | while IFS= read -r line; do
        printf "%04d | %s\n" "$line_number" "$line" >> "$OUTPUT"
        ((line_number++))
    done
}

if [[ -d "$DOCS_DIR" ]]; then
    echo "Including local docs/ files first..."
    find "$DOCS_DIR" -type f | sort | while read -r doc_file; do
        # Apply same exclusions
        for exclude in "${EXCLUDES[@]}"; do
            if [[ "$doc_file" == *"$exclude"* ]]; then
                echo "Skipping local doc: $doc_file (matched exclude: $exclude)"
                continue 2
            fi
        done
        add_header "$doc_file"
        nl -ba "$doc_file" >> "$OUTPUT"
    done
fi


# Step 2: Process remote repo files using for loop
echo "DEBUG: Starting remote file inclusion..."
FILES_PROCESSED=0
for file in $FILES; do
    # New exclude check
    for exclude in "${EXCLUDES[@]}"; do
        if [[ "$file" == *"$exclude"* ]]; then
            echo "Skipping: $file (matched exclude: $exclude)"
            continue 2
        fi
    done

    echo "Processing: $file"
    add_header "$file"
    add_file_content "$RAW_URL/$file"
    ((FILES_PROCESSED++))
done


echo "DEBUG: Processed $FILES_PROCESSED files"

# Insert TOC for Markdown
if [[ "$FORMAT" == "markdown" ]]; then
    TOC=$(cat "$TOC_TEMP")
    sed -i '' "s|(TOC will be inserted here)|$TOC|" "$OUTPUT" 2>/dev/null || \
    sed -i "s|(TOC will be inserted here)|$TOC|" "$OUTPUT"
fi

rm -f "$TOC_TEMP"
echo "✅ Combined file created: $OUTPUT"
