"""Build the M7 preview database from the real Steam candidate catalog."""

from __future__ import annotations

import shutil
import sqlite3
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DATA = ROOT / "data"
BASE = DATA / "m7-preview-rich-v4.db"
REAL = DATA / "m3-real.db"
MEDIA = DATA / "m7-live-smoke.db"
OUTPUT = DATA / "m7-preview-real-v1.db"
SAMPLE_APP_IDS = (2_500_001, 2_500_002)


def copy_rows(
    conn: sqlite3.Connection,
    source: str,
    table: str,
    *,
    columns: str = "*",
    conflict: str = "REPLACE",
) -> None:
    conn.execute(
        f"INSERT OR {conflict} INTO main.{table} SELECT {columns} FROM {source}.{table}"
    )


def delete_apps(conn: sqlite3.Connection, app_ids: tuple[int, ...]) -> None:
    placeholders = ",".join("?" for _ in app_ids)
    dependent_tables = [
        "app_relations",
        "app_localizations",
        "multiplayer_profiles",
        "feature_evidence",
        "curation_overrides",
        "review_snapshots",
        "player_snapshots",
        "player_daily",
        "price_snapshots",
        "release_events",
        "feedback_events",
        "app_availability",
        "play_intent_votes",
        "game_documents",
        "ai_analyses",
        "app_media",
        "popular_reviews",
    ]
    for table in dependent_tables:
        if table == "app_relations":
            conn.execute(
                f"DELETE FROM {table} WHERE source_app_id IN ({placeholders}) "
                f"OR target_app_id IN ({placeholders})",
                (*app_ids, *app_ids),
            )
        else:
            conn.execute(
                f"DELETE FROM {table} WHERE app_id IN ({placeholders})", app_ids
            )
    conn.execute(f"DELETE FROM apps WHERE app_id IN ({placeholders})", app_ids)


def build() -> None:
    for required in (BASE, REAL, MEDIA):
        if not required.exists():
            raise FileNotFoundError(required)

    shutil.copy2(BASE, OUTPUT)
    conn = sqlite3.connect(OUTPUT)
    try:
        conn.execute("PRAGMA foreign_keys = OFF")
        conn.execute("ATTACH DATABASE ? AS real", (str(REAL),))
        conn.execute("ATTACH DATABASE ? AS media", (str(MEDIA),))
        conn.execute("BEGIN IMMEDIATE")

        # Real catalog and snapshots replace older preview rows for matching AppIDs.
        for table in (
            "apps",
            "app_localizations",
            "multiplayer_profiles",
            "review_snapshots",
            "player_snapshots",
            "player_daily",
            "price_snapshots",
            "app_availability",
        ):
            copy_rows(conn, "real", table)

        # Evidence IDs and source-document IDs are database-local, so rebuild them
        # without retaining cross-database integer references.
        conn.execute("DELETE FROM feature_evidence")
        conn.execute(
            """
            INSERT INTO feature_evidence(
                app_id, feature_name, value_json, source_type, source_ref,
                source_document_id, confidence, observed_at_ms, expires_at_ms, is_active
            )
            SELECT app_id, feature_name, value_json, source_type, source_ref,
                   NULL, confidence, observed_at_ms, expires_at_ms, is_active
            FROM real.feature_evidence
            """
        )
        conn.execute("DELETE FROM source_documents")
        copy_rows(conn, "real", "source_documents")
        conn.execute("DELETE FROM source_runs")
        copy_rows(conn, "real", "source_runs")
        conn.execute("DELETE FROM source_cursors")
        copy_rows(conn, "real", "source_cursors")

        # The official catalog smoke database has current CDN capsule fallbacks for
        # nearly all discovered games. Preserve richer preview covers on conflicts.
        conn.execute(
            """
            INSERT OR IGNORE INTO app_media(app_id, capsule_url, source, updated_at_ms)
            SELECT media_row.app_id, media_row.capsule_url, media_row.source,
                   media_row.updated_at_ms
            FROM media.app_media media_row
            JOIN main.apps app ON app.app_id = media_row.app_id
            WHERE media_row.capsule_url IS NOT NULL
              AND TRIM(media_row.capsule_url) <> ''
            """
        )

        delete_apps(conn, SAMPLE_APP_IDS)
        conn.execute("DELETE FROM ai_analysis_cache")
        conn.execute("DELETE FROM game_embeddings")
        conn.execute("DELETE FROM game_documents")
        conn.execute("DELETE FROM game_fts")
        conn.execute("COMMIT")

        violations = conn.execute("PRAGMA foreign_key_check").fetchall()
        if violations:
            raise RuntimeError(f"foreign key violations: {violations[:10]}")
        integrity = conn.execute("PRAGMA integrity_check").fetchone()[0]
        if integrity != "ok":
            raise RuntimeError(f"integrity check failed: {integrity}")

        print(
            {
                "output": str(OUTPUT),
                "apps": conn.execute("SELECT COUNT(*) FROM apps").fetchone()[0],
                "profiles": conn.execute(
                    "SELECT COUNT(*) FROM multiplayer_profiles"
                ).fetchone()[0],
                "reviews": conn.execute(
                    "SELECT COUNT(DISTINCT app_id) FROM review_snapshots"
                ).fetchone()[0],
                "covers": conn.execute(
                    "SELECT COUNT(*) FROM app_media WHERE capsule_url IS NOT NULL"
                ).fetchone()[0],
                "popular_review_games": conn.execute(
                    "SELECT COUNT(DISTINCT app_id) FROM popular_reviews"
                ).fetchone()[0],
                "samples": conn.execute(
                    "SELECT COUNT(*) FROM apps WHERE app_id IN (?, ?)", SAMPLE_APP_IDS
                ).fetchone()[0],
                "integrity": integrity,
            }
        )
    finally:
        conn.close()


if __name__ == "__main__":
    try:
        build()
    except Exception as error:
        print(f"failed: {error}", file=sys.stderr)
        raise
