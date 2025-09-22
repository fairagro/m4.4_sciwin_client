$ErrorActionPreference = "Stop"

$downloadUrl = "https://github.com/fairagro/m4.4_sciwin_client/releases/latest/download/s4n-installer.sh"

powershell -ExecutionPolicy Bypass -c "irm $downloadUrl | iex"