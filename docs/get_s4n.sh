#!/bin/sh

set -u

url="https://api.github.com/repos/fairagro/m4.4_sciwin_client/releases/latest"

if [ -n "${GITHUB_TOKEN:-}" ]; then
    tag=$(curl -s -H "Authorization: token $GITHUB_TOKEN" "$url" | jq -r ".tag_name")
else
    tag=$(curl -s "$url" | jq -r ".tag_name")
fi

if [ -z "$tag" ] || [ "$tag" = "null" ]; then
    echo "Invalid Response, you may hit GitHub's Rate Limit, try: export GITHUB_TOKEN=\"ghp_your_actual_token\""
    return 1
fi

echo "Latest Release: ${tag}"
download_url="https://github.com/fairagro/m4.4_sciwin_client/releases/download/${tag}/s4n-installer.sh"

curl --proto '=https' --tlsv1.2 -LsSf $download_url | sh