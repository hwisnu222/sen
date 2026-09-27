#!/bin/bash

set -euo pipefail

APP_NAME="sen"
USERNAME="hwisnu222"
REPOSITORY="https://api.github.com/repos/$USERNAME/$APP_NAME/releases/latest"

if [ -n "${TERMUX_VERSION:-}" ] || [[ "${PREFIX:-}" == *com.termux* ]]; then
  IS_TERMUX=true
  INSTALL_DIR="$PREFIX/bin"
  echo "System detected: Termux (aarch64)"
else
  IS_TERMUX=false
  INSTALL_DIR="/usr/local/bin"

  # non-Termux
  if [[ $EUID -ne 0 ]]; then
    echo "Please run command with sudo"
    exit 1
  fi

  echo "System detected: Linux ($(uname -m))"
fi

if [ "$IS_TERMUX" = true ]; then
  ASSET_SUFFIX="android-aarch64"
else
  ARCH_S=$(uname -m)
  ASSET_SUFFIX="$ARCH_S"
fi

if ! VERSION=$(curl -sL $REPOSITORY | jq -r ".tag_name"); then
  echo "Failed to get tag repository"
  exit 1
fi

URL_RELEASE="https://github.com/$USERNAME/$APP_NAME/releases/download/$VERSION/$APP_NAME-$ASSET_SUFFIX"

echo "Downloading binary from: $URL_RELEASE"
if curl -L "$URL_RELEASE" -o "$INSTALL_DIR/$APP_NAME"; then
  chmod +x "$INSTALL_DIR/$APP_NAME"
  echo "Install finished successfully to $INSTALL_DIR/$APP_NAME"
else
  echo "Failed to download binary file"
  exit 1
fi
