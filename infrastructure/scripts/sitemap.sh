curl -s https://api.github.com/repos/USERNAME/REPO/git/trees/BRANCH?recursive=1 \
| jq -r '.tree[] | select(.type=="blob") | .path' \
| sed 's|^|https://github.com/USERNAME/REPO/blob/BRANCH/|' > sitemap.md
