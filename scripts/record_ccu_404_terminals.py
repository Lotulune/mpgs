#!/usr/bin/env python3
"""Persist official CCU HTTP-404 outcomes already captured in an enrichment log."""

from __future__ import annotations

import argparse
import hashlib
import re
import sqlite3
import time
from pathlib import Path

CCU_404 = re.compile(r"warn app_id=(\d+) ccu: HTTP status 404 is not successful")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", required=True)
    parser.add_argument("--log", required=True)
    args = parser.parse_args()

    log_path = Path(args.log)
    app_ids = sorted({int(value) for value in CCU_404.findall(log_path.read_text(encoding="utf-8"))})
    now_ms = int(time.time() * 1000)
    conn = sqlite3.connect(args.db, timeout=30)
    conn.execute("PRAGMA foreign_keys = ON")
    written = 0
    with conn:
        for app_id in app_ids:
            exists = conn.execute("SELECT 1 FROM apps WHERE app_id = ?", (app_id,)).fetchone()
            if not exists:
                continue
            content_hash = hashlib.sha256(f"ccu-http-404:{app_id}".encode()).hexdigest()
            conn.execute(
                """
                INSERT OR REPLACE INTO player_snapshots(
                    app_id, captured_at_ms, player_count, result_code, missing_reason,
                    content_hash, source, offline_players_excluded
                ) VALUES (?, ?, NULL, 404, 'endpoint_http_not_found', ?,
                          'steam_userstats_current_players', 1)
                """,
                (app_id, now_ms, content_hash),
            )
            written += 1
    print(f"ccu_404_log_ids={len(app_ids)} written={written}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
