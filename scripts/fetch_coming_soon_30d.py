#!/usr/bin/env python3
"""Fetch Steam multiplayer coming-soon titles and keep those releasing within 30 days.

Writes into a local MPGS SQLite database (apps + app_media + category_hint evidence).
Does not overwrite existing `released` apps with coming_soon state.
"""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
import time
import urllib.parse
import urllib.request
from datetime import date, datetime, timedelta, timezone
from typing import Any

UA = "MPGS-Server/0.1 (+https://github.com/Lotulune/mpgs; research)"
STORE = "https://store.steampowered.com"
MONTHS = {
    "jan": 1,
    "january": 1,
    "feb": 2,
    "february": 2,
    "mar": 3,
    "march": 3,
    "apr": 4,
    "april": 4,
    "may": 5,
    "jun": 6,
    "june": 6,
    "jul": 7,
    "july": 7,
    "aug": 8,
    "august": 8,
    "sep": 9,
    "sept": 9,
    "september": 9,
    "oct": 10,
    "october": 10,
    "nov": 11,
    "november": 11,
    "dec": 12,
    "december": 12,
}


def http_get(url: str, timeout: float = 30.0) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": UA, "Accept": "application/json,*/*"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return resp.read()


def normalize_release_date(raw: str | None) -> tuple[str | None, str | None]:
    if not raw:
        return None, None
    text = raw.strip()
    if not text or text.upper() in {"TBA", "TBD", "COMING SOON"}:
        return None, "tba"
    m = re.fullmatch(r"(\d{4})-(\d{2})-(\d{2})", text)
    if m:
        return text, "day"
    m = re.fullmatch(r"(\d{4})\s*年\s*(\d{1,2})\s*月\s*(\d{1,2})\s*日?", text)
    if m:
        year, month, day = int(m.group(1)), int(m.group(2)), int(m.group(3))
        try:
            return date(year, month, day).isoformat(), "day"
        except ValueError:
            return None, "tba"
    m = re.fullmatch(
        r"(?i)(jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|"
        r"jul(?:y)?|aug(?:ust)?|sep(?:t(?:ember)?)?|oct(?:ober)?|nov(?:ember)?|"
        r"dec(?:ember)?)\s+(\d{1,2}),\s*(\d{4})",
        text,
    )
    if m:
        month = MONTHS[m.group(1).lower()]
        day = int(m.group(2))
        year = int(m.group(3))
        try:
            return date(year, month, day).isoformat(), "day"
        except ValueError:
            return None, "tba"
    m = re.fullmatch(r"(?i)(q[1-4])\s+(\d{4})", text)
    if m:
        q = int(m.group(1)[1])
        year = int(m.group(2))
        month = (q - 1) * 3 + 1
        return f"{year:04d}-{month:02d}-01", "quarter"
    m = re.fullmatch(r"(?i)(\d{4})", text)
    if m:
        return f"{int(m.group(1)):04d}-01-01", "year"
    return None, "tba"


def search_coming_soon(start: int, count: int = 50) -> dict[str, Any]:
    qs = urllib.parse.urlencode(
        {
            "query": "",
            "start": start,
            "count": count,
            "dynamic_data": "",
            "sort_by": "Released_ASC",
            "snr": "1_7_7_230_7",
            "filter": "comingsoon",
            "category1": "998",
            "category2": "1",
            "infinite": "1",
            "cc": "CN",
            "l": "schinese",
            "json": "1",
        }
    )
    raw = http_get(f"{STORE}/search/results/?{qs}")
    return json.loads(raw.decode("utf-8", errors="replace"))


def parse_search_rows(html: str) -> list[tuple[int, str]]:
    rows: list[tuple[int, str]] = []
    for match in re.finditer(
        r'data-ds-appid="(\d+)[^"]*"[^>]*>[\s\S]*?<span class="title">([^<]+)</span>',
        html,
        flags=re.I,
    ):
        app_id = int(match.group(1))
        name = re.sub(r"\s+", " ", match.group(2)).strip()
        if app_id and name:
            rows.append((app_id, name))
    # de-dupe preserve order
    seen: set[int] = set()
    out: list[tuple[int, str]] = []
    for app_id, name in rows:
        if app_id in seen:
            continue
        seen.add(app_id)
        out.append((app_id, name))
    return out


def fetch_appdetails(app_id: int) -> dict[str, Any] | None:
    qs = urllib.parse.urlencode({"appids": app_id, "cc": "CN", "l": "schinese"})
    raw = http_get(f"{STORE}/api/appdetails?{qs}")
    payload = json.loads(raw.decode("utf-8", errors="replace"))
    entry = payload.get(str(app_id))
    if not isinstance(entry, dict) or not entry.get("success"):
        return None
    data = entry.get("data")
    return data if isinstance(data, dict) else None


def upsert_coming_soon(
    conn: sqlite3.Connection,
    *,
    app_id: int,
    name: str,
    release_date: str | None,
    release_date_raw: str | None,
    release_date_precision: str | None,
    header_image: str | None,
    now_ms: int,
) -> None:
    existing = conn.execute(
        "SELECT release_state FROM apps WHERE app_id = ?",
        (app_id,),
    ).fetchone()
    if existing and existing[0] == "released":
        # Do not demote a released catalog entry.
        return

    conn.execute(
        """
        INSERT INTO apps (
            app_id, app_type, canonical_name, release_state, release_date,
            release_date_raw, release_date_precision, source_modified_at_ms,
            created_at_ms, updated_at_ms
        ) VALUES (?, 'game', ?, 'coming_soon', ?, ?, ?, ?, ?, ?)
        ON CONFLICT(app_id) DO UPDATE SET
            app_type = 'game',
            canonical_name = excluded.canonical_name,
            release_state = CASE
                WHEN apps.release_state = 'released' THEN apps.release_state
                ELSE 'coming_soon'
            END,
            release_date = COALESCE(excluded.release_date, apps.release_date),
            release_date_raw = COALESCE(excluded.release_date_raw, apps.release_date_raw),
            release_date_precision = COALESCE(excluded.release_date_precision, apps.release_date_precision),
            source_modified_at_ms = excluded.source_modified_at_ms,
            updated_at_ms = excluded.updated_at_ms
        """,
        (
            app_id,
            name,
            release_date,
            release_date_raw,
            release_date_precision,
            now_ms,
            now_ms,
            now_ms,
        ),
    )

    capsule = header_image or f"https://cdn.akamai.steamstatic.com/steam/apps/{app_id}/header.jpg"
    conn.execute(
        """
        INSERT INTO app_media (app_id, capsule_url, source, updated_at_ms)
        VALUES (?, ?, 'steam_store_appdetails', ?)
        ON CONFLICT(app_id) DO UPDATE SET
            capsule_url = COALESCE(excluded.capsule_url, app_media.capsule_url),
            source = excluded.source,
            updated_at_ms = excluded.updated_at_ms
        """,
        (app_id, capsule, now_ms),
    )

    evidence = json.dumps(
        {"category": "Multi-player", "filter": "comingsoon+category2=1"},
        ensure_ascii=False,
    )
    conn.execute(
        """
        INSERT INTO feature_evidence (
            app_id, feature_name, value_json, source_type, source_ref,
            confidence, is_active, observed_at_ms
        ) VALUES (?, 'category_hint', ?, 'store_search_category',
                  'steam_store_search:comingsoon', 0.35, 1, ?)
        """,
        (app_id, evidence, now_ms),
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--db", default="data/m7-preview-real-v1.db")
    parser.add_argument("--days", type=int, default=30)
    parser.add_argument("--max-search", type=int, default=200, help="max search rows to inspect")
    parser.add_argument("--sleep", type=float, default=1.1)
    args = parser.parse_args()

    today = date.today()
    end = today + timedelta(days=args.days)
    now_ms = int(datetime.now(tz=timezone.utc).timestamp() * 1000)

    print(f"window {today.isoformat()} .. {end.isoformat()} db={args.db}")
    conn = sqlite3.connect(args.db)
    conn.execute("PRAGMA foreign_keys = ON")

    candidates: list[tuple[int, str]] = []
    start = 0
    total = None
    while len(candidates) < args.max_search:
        page = search_coming_soon(start=start, count=50)
        if total is None:
            total = int(page.get("total_count") or 0)
            print(f"steam comingsoon multiplayer total_count={total}")
        html = page.get("results_html") or ""
        rows = parse_search_rows(html)
        if not rows:
            break
        candidates.extend(rows)
        start += len(rows)
        if start >= (total or 0):
            break
        time.sleep(args.sleep)

    # unique
    seen: set[int] = set()
    unique: list[tuple[int, str]] = []
    for app_id, name in candidates:
        if app_id in seen:
            continue
        seen.add(app_id)
        unique.append((app_id, name))
    unique = unique[: args.max_search]
    print(f"inspecting {len(unique)} search candidates")

    kept = 0
    skipped = 0
    for idx, (app_id, search_name) in enumerate(unique, start=1):
        try:
            details = fetch_appdetails(app_id)
        except Exception as exc:  # noqa: BLE001
            print(f"[{idx}/{len(unique)}] {app_id} appdetails failed: {exc}")
            skipped += 1
            time.sleep(args.sleep)
            continue
        time.sleep(args.sleep)
        if not details:
            skipped += 1
            continue
        release = details.get("release_date") or {}
        coming_soon = bool(release.get("coming_soon"))
        raw = release.get("date") if isinstance(release.get("date"), str) else None
        iso, precision = normalize_release_date(raw)
        name = details.get("name") if isinstance(details.get("name"), str) else search_name
        header = details.get("header_image") if isinstance(details.get("header_image"), str) else None

        in_window = False
        if iso:
            try:
                d = date.fromisoformat(iso)
                in_window = today <= d <= end
            except ValueError:
                in_window = False

        # Keep undated coming-soon only if Steam still marks coming_soon and we are
        # within the first N search hits (calendar undated section).
        if coming_soon and (in_window or iso is None):
            if iso is None or in_window:
                upsert_coming_soon(
                    conn,
                    app_id=app_id,
                    name=name,
                    release_date=iso if in_window else None,
                    release_date_raw=raw,
                    release_date_precision=precision if in_window else (precision or "tba"),
                    header_image=header,
                    now_ms=now_ms,
                )
                kept += 1
                print(f"[{idx}] keep {app_id} {name!r} date={iso or raw!r}")
            else:
                skipped += 1
        else:
            skipped += 1

        if idx % 20 == 0:
            conn.commit()

    conn.commit()
    count = conn.execute(
        "SELECT count(*) FROM apps WHERE release_state IN ('upcoming','coming_soon')"
    ).fetchone()[0]
    dated = conn.execute(
        "SELECT count(*) FROM apps WHERE release_state IN ('upcoming','coming_soon') "
        "AND release_date BETWEEN ? AND ?",
        (today.isoformat(), end.isoformat()),
    ).fetchone()[0]
    print(f"done kept={kept} skipped={skipped} coming_soon_total={count} dated_in_window={dated}")
    conn.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
