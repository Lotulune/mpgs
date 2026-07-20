#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)
branch=${MPGS_DEPLOY_BRANCH:-main}

cd "$repo_root"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git pull --ff-only origin "$branch"
else
  printf 'Source directory is not a Git checkout; pulling container images only.\n'
fi

docker compose --env-file deploy/.env -f deploy/docker-compose.yml pull
docker compose --env-file deploy/.env -f deploy/docker-compose.yml \
  up -d --no-build --remove-orphans

docker compose --env-file deploy/.env -f deploy/docker-compose.yml \
  exec -T mpgs-server mpgs-dbtool integrity /var/lib/mpgs/mpgs.db
curl --fail --silent --show-error http://127.0.0.1:18082/health/ready
printf '\nMPGS deployment updated from %s\n' "$branch"
