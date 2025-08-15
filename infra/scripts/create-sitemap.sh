#!/usr/bin/env bash

set -euo pipefail

USER=$1
REPO=$2
BRANCH=${3:-""}
shift 2
EXCLUDES=("${@:2}")

if [[ -z "$USER" || -z "$REPO" ]]; then
    echo "❌ Usage: $0 <github_user> <repo_name> [branch] [exclude1 exclude2 ...]"
    exit 1
fi

# Auto-detect branch if not provided
if [[ -z "$BRANCH" ]]; then
    BRANCH=$(curl -s "https://api.github.com/repos/$USER/$REPO" | jq -r '.default_branch')
fi

# Always exclude .env
EXCLUDES+=(".env")
EXCLUDE_REGEX=""
if [[ ${#EXCLUDES[@]} -gt 0 ]]; then
    EXCLUDE_REGEX=$(printf "|%s" "${EXCLUDES[@]}")
    EXCLUDE_REGEX=${EXCLUDE_REGEX:1}
fi

API_URL="https://api.github.com/repos/$USER/$REPO/git/trees/$BRANCH?recursive=1"
echo "Fetching file list from GitHub API for branch '$BRANCH'..."

FILE_LIST=$(curl -s "$API_URL" | jq -r '.tree? | map(select(.type=="blob")) | .[]?.path')
COUNT=$(echo "$FILE_LIST" | wc -l)
echo "DEBUG: Fetched $COUNT files from $REPO ($BRANCH)"

if [[ $COUNT -eq 0 ]]; then
    echo "⚠ No files found. Generating empty docs page."
fi

# Apply exclusions
if [[ -n "$EXCLUDE_REGEX" ]]; then
    FILE_LIST=$(echo "$FILE_LIST" | grep -Ev "^($EXCLUDE_REGEX)(/|$)" || true)
fi

mkdir -p docs
TIMESTAMP=$(date -u "+%Y-%m-%d %H:%M UTC")

########################################
# Always create index.html & robots.txt
########################################
cat <<EOF > docs/index.html
<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="robots" content="noindex, nofollow">
<title>$REPO Sitemap</title>
<style>
body { font-family: Arial, sans-serif; padding: 40px; }
h1 { color: #0366d6; }
a { font-size: 18px; text-decoration: none; color: #0366d6; display: block; margin: 10px 0; }
a:hover { text-decoration: underline; }
.timestamp { color: #555; font-style: italic; }
</style>
</head>
<body>
<h1>Sitemap for $REPO ($BRANCH)</h1>
<p class="timestamp">Generated on: $TIMESTAMP</p>
<a href="sitemap.html">View HTML Sitemap (Collapsible)</a>
<a href="sitemap.md">View Markdown Sitemap</a>
</body>
</html>
EOF

echo -e "User-agent: *\nDisallow: /" > docs/robots.txt

########################################
# If no files, exit after index.html
########################################
if [[ $COUNT -eq 0 ]]; then
    echo "✅ Created docs/index.html and robots.txt (empty sitemap)."
    exit 0
fi

########################################
# Generate Markdown Sitemap
########################################
echo "# Sitemap for $REPO ($BRANCH)" > docs/sitemap.md
echo "*Generated on: $TIMESTAMP*" >> docs/sitemap.md
echo "" >> docs/sitemap.md

declare -a open_dirs=()  # FIX: Pre-initialize to avoid unbound variable
while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    IFS='/' read -ra parts <<< "$path"
    depth=${#parts[@]}
    last_index=$((depth-1))

    for ((j=depth-1; j<${#open_dirs[@]}; j++)); do unset open_dirs[j]; done

    for ((i=0; i<depth-1; i++)); do
        dir="${parts[i]}"
        if [[ "${open_dirs[i]-}" != "$dir" ]]; then
            indent=$(printf '    %.0s' $(seq 1 $i))
            echo "$indent- **$dir/**" >> docs/sitemap.md
            open_dirs[i]="$dir"
        fi
    done

    indent=$(printf '    %.0s' $(seq 1 $((depth-1))))
    file="${parts[$last_index]}"
    echo "$indent- [$file](https://github.com/$USER/$REPO/blob/$BRANCH/$path)" >> docs/sitemap.md
done <<< "$FILE_LIST"

########################################
# Generate HTML Sitemap (Collapsible)
########################################
cat <<EOF > docs/sitemap.html
<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8">
<meta name="robots" content="noindex, nofollow">
<title>Sitemap for $REPO</title>
<style>
body { font-family: Arial, sans-serif; padding: 20px; }
ul { list-style-type: none; margin: 0; padding-left: 20px; }
li { margin: 4px 0; }
.folder { font-weight: bold; cursor: pointer; }
.hidden { display: none; }
a { text-decoration: none; color: #0366d6; }
a:hover { text-decoration: underline; }
.timestamp { font-style: italic; color: #666; margin-bottom: 15px; display: block; }
button { margin: 5px; }
</style>
<script>
function toggleFolder(el) {
    var next = el.nextElementSibling;
    if (next) { next.classList.toggle('hidden'); }
}
function toggleAll(expand) {
    var lists = document.querySelectorAll('ul');
    for (var i = 1; i < lists.length; i++) {
        if (expand) lists[i].classList.remove('hidden');
        else lists[i].classList.add('hidden');
    }
}
</script>
</head>
<body>
<h1>Sitemap for $REPO ($BRANCH)</h1>
<span class="timestamp">Generated on: $TIMESTAMP</span>
<button onclick="toggleAll(true)">Expand All</button>
<button onclick="toggleAll(false)">Collapse All</button>
<ul>
EOF

prev_depth=0
while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    IFS='/' read -ra parts <<< "$path"
    depth=${#parts[@]}
    last_index=$((depth-1))

    while [[ $prev_depth -gt $((depth-1)) ]]; do
        echo "</ul></li>" >> docs/sitemap.html
        ((prev_depth--))
    done

    for ((i=prev_depth; i<depth-1; i++)); do
        echo "<li><span class='folder' onclick='toggleFolder(this)'>📂 ${parts[i]}</span><ul class='hidden'>" >> docs/sitemap.html
    done

    file="${parts[$last_index]}"
    echo "<li><a href=\"https://github.com/$USER/$REPO/blob/$BRANCH/$path\">$file</a></li>" >> docs/sitemap.html

    prev_depth=$((depth-1))
done <<< "$FILE_LIST"

while [[ $prev_depth -gt 0 ]]; do
    echo "</ul></li>" >> docs/sitemap.html
    ((prev_depth--))
done

cat <<EOF >> docs/sitemap.html
</ul>
</body>
</html>
EOF

echo "✅ Created sitemap.md, sitemap.html, index.html, robots.txt in docs/"
