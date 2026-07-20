#!/bin/sh
set -eu

interval="${MPGS_WORKER_INTERVAL_SECS:-60}"
job_limit="${MPGS_WORKER_JOB_LIMIT:-10}"
enrich_limit="${MPGS_WORKER_ENRICH_LIMIT:-100}"

while :; do
    /usr/local/bin/mpgs-dbtool run-steam-worker-once \
        /var/lib/mpgs/mpgs.db "$job_limit" "$enrich_limit" || true
    sleep "$interval"
done
