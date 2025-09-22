#!/bin/sh

set -eu

curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fairagro/m4.4_sciwin_client/releases/latest/download/s4n-installer.sh | sh