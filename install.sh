#!/bin/bash

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  echo "Please run command with sudo"
  exit 1
fi

ARCH_S=$(uname -m)
echo "System detected: $ARCH_S"

APP_NAME="sen"
USERNAME="hwisnu222"
REPOSITORY="https://api.github.com/repos/$USERNAME/$APP_NAME/releases/latest"

if ! VERSION=$(curl -sL $REPOSITORY | jq -r ".tag_name"); then
  echo "failed get tag repository"
  exit 1
fi

URL_RELEASE="https://github.com/$USERNAME/$APP_NAME/releases/download/$VERSION/$APP_NAME-$ARCH_S"

echo "Download binary..."
if curl -L $URL_RELEASE -o /usr/local/bin/$APP_NAME; then
  chmod +x /usr/local/bin/$APP_NAME
  echo "Install finished"
else
  echo "failed download binary file"
  exit 1
fi
