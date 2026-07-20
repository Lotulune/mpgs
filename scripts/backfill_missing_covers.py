#!/usr/bin/env python3
"""Backfill localized store summaries and real Steam covers via appdetails."""

from __future__ import annotations

import argparse
import json
import sqlite3
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone

UA = "MPGS-Server/0.1 (+https://github.com/Lotulune/mpgs; research)"


def verified_catalog_header(app_id: int) -> str | None:
    """Return Steam's canonical header URL only after the CDN confirms an image."""
    url = f"https://cdn.akamai.steamstatic.com/steam/apps/{app_id}/header.jpg"
    req = urllib.request.Request(url, headers={"User-Agent": UA}, method="HEAD")
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            content_type = (resp.headers.get("Content-Type") or "").lower()
            if resp.status == 200 and content_type.startswith("image/"):
                return url
    except (urllib.error.HTTPError, urllib.error.URLError, TimeoutError):
        return None
    return None


def fetch_details(
    app_id: int, *, country: str, language: str
) -> tuple[str | None, str | None, str | None]:
    query = urllib.parse.urlencode(
        {"appids": app_id, "cc": country, "l": language}
    )
    url = f"https://store.steampowered.com/api/appdetails?{query}"
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=25) as resp:
        payload = json.loads(resp.read().decode("utf-8", errors="replace"))
    entry = payload.get(str(app_id))
    if not isinstance(entry, dict) or not entry.get("success"):
        return verified_catalog_header(app_id), None, None
    data = entry.get("data")
    if not isinstance(data, dict):
        return verified_catalog_header(app_id), None, None
    header = data.get("header_image")
    name = data.get("name")
    summary = data.get("short_description")
    return (
        header
        if isinstance(header, str) and header.strip()
        else verified_catalog_header(app_id),
        name if isinstance(name, str) and name.strip() else None,
        summary if isinstance(summary, str) and summary.strip() else None,
    )


def fetch_details_with_retry(
    app_id: int,
    *,
    retries: int,
    retry_base: float,
    rate_limit_sleep: float,
    country: str,
    language: str,
) -> tuple[str | None, str | None, str | None]:
    for attempt in range(retries + 1):
        try:
            return fetch_details(app_id, country=country, language=language)
        except Exception as exc:  # noqa: BLE001
            if attempt >= retries:
                raise
            delay = retry_base * (2**attempt)
            if isinstance(exc, urllib.error.HTTPError) and exc.code == 429:
                retry_after = exc.headers.get("Retry-After")
                try:
                    retry_after_seconds = float(retry_after or 0)
                except ValueError:
                    retry_after_seconds = 0.0
                delay = max(rate_limit_sleep, retry_after_seconds)
            print(
                f"retry app_id={app_id} attempt={attempt + 1}/{retries} "
                f"in={delay:.1f}s error={exc}"
            )
            time.sleep(delay)
    raise AssertionError("retry loop must return or raise")


def main() -> int:
    # Windows may default redirected output to GBK. Steam titles can contain
    # characters outside that code page; logging must never abort a data job.
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            stream.reconfigure(
                encoding="utf-8",
                errors="replace",
                line_buffering=True,
                write_through=True,
            )

    parser = argparse.ArgumentParser()
    parser.add_argument("--db", default="data/m7-preview-real-v1.db")
    parser.add_argument("--sleep", type=float, default=1.0)
    parser.add_argument("--limit", type=int, default=0, help="0 = all missing")
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--retry-base", type=float, default=5.0)
    parser.add_argument("--rate-limit-sleep", type=float, default=300.0)
    parser.add_argument("--country", default="CN")
    parser.add_argument("--language", default="schinese")
    parser.add_argument("--circuit-failures", type=int, default=3)
    parser.add_argument("--circuit-sleep", type=float, default=120.0)
    parser.add_argument(
        "--app-ids",
        default="",
        help="comma-separated AppIDs to backfill before the remaining catalog",
    )
    args = parser.parse_args()

    now_ms = int(datetime.now(tz=timezone.utc).timestamp() * 1000)
    conn = sqlite3.connect(args.db)
    country = args.country.strip().upper()
    language = args.language.strip().lower()
    if len(country) != 2 or not country.isalpha():
        parser.error("--country must be a two-letter country code")
    if not language or not language.replace("_", "").isalnum():
        parser.error("--language must be a Steam language identifier")
    rows = conn.execute(
        """
        SELECT a.app_id, a.canonical_name
        FROM apps a
        LEFT JOIN app_media m ON m.app_id = a.app_id
        LEFT JOIN app_localizations l
          ON l.app_id = a.app_id AND l.language = ?
        WHERE a.release_state = 'released'
          AND (
              m.capsule_url IS NULL OR TRIM(m.capsule_url) = ''
              OR m.source = 'steam_catalog'
              OR l.app_id IS NULL
          )
        ORDER BY COALESCE(a.release_date, '0000-00-00') DESC, a.app_id
        """,
        (language,),
    ).fetchall()
    requested_ids = []
    for value in args.app_ids.split(","):
        value = value.strip()
        if value:
            requested_ids.append(int(value))
    if requested_ids:
        by_id = {app_id: (app_id, name) for app_id, name in rows}
        requested = [by_id[app_id] for app_id in requested_ids if app_id in by_id]
        requested_set = {app_id for app_id, _ in requested}
        rows = requested + [row for row in rows if row[0] not in requested_set]
    if args.limit > 0:
        rows = rows[: args.limit]
    print(f"missing covers: {len(rows)}")
    ok = 0
    fail = 0
    consecutive_failures = 0
    for idx, (app_id, name) in enumerate(rows, start=1):
        try:
            header, localized_name, summary = fetch_details_with_retry(
                app_id,
                retries=max(args.retries, 0),
                retry_base=max(args.retry_base, 0.0),
                rate_limit_sleep=max(args.rate_limit_sleep, 0.0),
                country=country,
                language=language,
            )
        except Exception as exc:  # noqa: BLE001
            print(f"[{idx}/{len(rows)}] fail {app_id} {name!r}: {exc}")
            fail += 1
            consecutive_failures += 1
            if consecutive_failures >= max(args.circuit_failures, 1):
                conn.commit()
                print(
                    f"circuit-open consecutive_failures={consecutive_failures} "
                    f"sleep={max(args.circuit_sleep, 0.0):.1f}s"
                )
                time.sleep(max(args.circuit_sleep, 0.0))
                consecutive_failures = 0
            time.sleep(args.sleep)
            continue
        consecutive_failures = 0
        if not header and not summary:
            print(f"[{idx}/{len(rows)}] no-details {app_id} {name!r}")
            fail += 1
            time.sleep(args.sleep)
            continue
        if header:
            conn.execute(
                """
                INSERT INTO app_media (app_id, capsule_url, source, updated_at_ms)
                VALUES (?, ?, 'steam_store_appdetails', ?)
                ON CONFLICT(app_id) DO UPDATE SET
                    capsule_url = excluded.capsule_url,
                    source = excluded.source,
                    updated_at_ms = excluded.updated_at_ms
                """,
                (app_id, header, now_ms),
            )
        if localized_name or summary:
            conn.execute(
                """
                INSERT INTO app_localizations(
                    app_id, language, name, short_description, source, updated_at_ms
                ) VALUES (?, ?, ?, ?, 'steam_store_appdetails', ?)
                ON CONFLICT(app_id, language) DO UPDATE SET
                    name = COALESCE(excluded.name, app_localizations.name),
                    short_description = COALESCE(
                        excluded.short_description,
                        app_localizations.short_description
                    ),
                    source = excluded.source,
                    updated_at_ms = excluded.updated_at_ms
                """,
                (app_id, language, localized_name, summary, now_ms),
            )
        ok += 1
        print(f"[{idx}/{len(rows)}] ok {app_id} {name!r}")
        if idx % 10 == 0:
            conn.commit()
        time.sleep(args.sleep)
    conn.commit()
    print(f"done ok={ok} fail={fail}")
    conn.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
