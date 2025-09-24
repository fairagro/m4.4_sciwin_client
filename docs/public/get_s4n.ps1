$ErrorActionPreference = "Stop"

$downloadUrl = "https://github.com/fairagro/m4.4_sciwin_client/releases/latest/download/s4n-installer.ps1"

powershell -ExecutionPolicy Bypass -c "irm $downloadUrl | iex"