#!/usr/bin/env bash
# Deploy the gpm-sync server to the VPS.
# Reads VPS_HOST and DEPLOY_DIR from server/.env.prod, ships the server/
# directory + .env.prod (as .env), then rebuilds and restarts the container.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SERVER_DIR="$PROJECT_ROOT/server"
ENV_FILE="$SERVER_DIR/.env.prod"

if [ ! -f "$ENV_FILE" ]; then
  echo "missing $ENV_FILE (copy from .env.example and fill in)" >&2
  exit 1
fi

set -a
source "$ENV_FILE"
set +a

: "${VPS_HOST:?VPS_HOST not set in .env.prod}"
: "${DEPLOY_DIR:?DEPLOY_DIR not set in .env.prod}"

echo "→ syncing server/ to $VPS_HOST:$DEPLOY_DIR"
ssh "$VPS_HOST" "mkdir -p $DEPLOY_DIR"
rsync -avz --delete \
  --exclude='target' \
  --exclude='.env' \
  --exclude='.env.prod' \
  "$SERVER_DIR/" "$VPS_HOST:$DEPLOY_DIR/"

echo "→ copying .env.prod → remote .env"
scp "$ENV_FILE" "$VPS_HOST:$DEPLOY_DIR/.env"

echo "→ building and restarting container"
ssh "$VPS_HOST" "cd $DEPLOY_DIR && docker compose up -d --build"

echo "→ status"
ssh "$VPS_HOST" "cd $DEPLOY_DIR && docker compose ps"
