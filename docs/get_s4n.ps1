$ErrorActionPreference = "Stop"

$url = "https://api.github.com/repos/fairagro/m4.4_sciwin_client/releases/latest"

if ($env:GITHUB_TOKEN) {
    $tag = (Invoke-RestMethod -Uri $url -Headers @{Authorization = "token $($env:GITHUB_TOKEN)"}).tag_name
} else {
    $tag = (Invoke-RestMethod -Uri $url).tag_name
}

if ([string]::IsNullOrEmpty($tag) -or $tag -eq "null") {
    Write-Host "Invalid Response, you may hit GitHub's Rate Limit, try: $env:GITHUB_TOKEN = 'ghp_your_actual_token'"
    return
}

Write-Host "Latest Release: $tag"

$downloadUrl = "https://github.com/fairagro/m4.4_sciwin_client/releases/download/$tag/s4n-installer.ps1"

powershell -ExecutionPolicy Bypass -c "irm $downloadUrl | iex"