#![forbid(unsafe_code)]

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use mpgs_steam_source::{
    DEFAULT_USER_AGENT, RawResponse, STEAM_STORE_HOST, STORE_SEARCH_ADAPTER_VERSION,
    STORE_SEARCH_SOURCE_NAME, SourceError, StoreSearchPage, StoreSearchRequest,
    parse_store_search_page,
};
use mpgs_storage::{Clock, Database, Repository, SystemClock};
use serde::{Deserialize, Serialize};

const STORE_SEARCH_CURSOR_KEY: &str = "steam_store_search:multiplayer:reviews_desc";
const STORE_SEARCH_TARGET_DEFAULT: u32 = 2_000;
const STORE_SEARCH_TARGET_MAX: u32 = 10_000;
const STORE_SEARCH_RESPONSE_MAX_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoreSearchCursor {
    next_start: u32,
    total_count: Option<u32>,
    target: u32,
    complete: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct CollectionStats {
    request_count: i64,
    success_count: i64,
}

#[derive(Debug)]
struct CollectionError {
    category: &'static str,
    message: String,
    stats: CollectionStats,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let cmd = args.next().ok_or_else(|| usage().to_owned())?;

    match cmd.as_str() {
        "migrate" => {
            let db_path = required_path(args.next(), "--db path")?;
            let db = Database::open(&db_path).map_err(err)?;
            let version = db.migrate().map_err(err)?;
            println!("migrated {} to schema version {version}", db_path.display());
            db.assert_ready().map_err(err)?;
            Ok(())
        }
        "integrity" => {
            let db_path = required_path(args.next(), "--db path")?;
            let db = Database::open(&db_path).map_err(err)?;
            let check = db.integrity_check().map_err(err)?;
            let version = db.schema_version().map_err(err)?;
            println!("path={}", db_path.display());
            println!("schema_version={version}");
            println!("integrity_check={check:?}");
            db.assert_ready().map_err(err)?;
            println!("ready=ok");
            Ok(())
        }
        "m3-audit" => {
            let db_path = required_path(args.next(), "--db path")?;
            let db = Database::open(&db_path).map_err(err)?;
            db.assert_ready().map_err(err)?;
            let repo = Repository::new(db);
            repo.readiness_check().map_err(err)?;
            audit_m3(&repo, &db_path)
        }
        "collect-steam-candidates" => {
            let db_path = required_path(args.next(), "--db path")?;
            let target = optional_target(args.next())?;
            if args.next().is_some() {
                return Err("collect-steam-candidates accepts at most db-path and target".into());
            }
            let db = Database::open(&db_path).map_err(err)?;
            db.migrate().map_err(err)?;
            let repo = Repository::new(db);
            repo.ensure_runtime_defaults().map_err(err)?;
            let before = repo.m3_catalog_coverage().map_err(err)?;
            if before.normalized_multiplayer_candidates >= i64::from(target) {
                println!("path={}", db_path.display());
                println!(
                    "normalized_multiplayer_candidates={}",
                    before.normalized_multiplayer_candidates
                );
                println!("steam_candidate_collection=already_satisfied");
                return Ok(());
            }

            let run_id = repo
                .start_source_run(
                    STORE_SEARCH_SOURCE_NAME,
                    "candidate_discovery",
                    STORE_SEARCH_ADAPTER_VERSION,
                    Some("category2=1;sort=Reviews_DESC;cc=US;l=english"),
                )
                .map_err(err)?;
            match collect_steam_candidates(&repo, target) {
                Ok(stats) => {
                    repo.finish_source_run(
                        run_id,
                        "succeeded",
                        stats.request_count,
                        stats.success_count,
                        None,
                        Some("target reached"),
                    )
                    .map_err(err)?;
                    let after = repo.m3_catalog_coverage().map_err(err)?;
                    println!("path={}", db_path.display());
                    println!("requests={}", stats.request_count);
                    println!("rows_ingested={}", stats.success_count);
                    println!(
                        "normalized_multiplayer_candidates={}",
                        after.normalized_multiplayer_candidates
                    );
                    println!("steam_candidate_collection=ok");
                    Ok(())
                }
                Err(failure) => {
                    let status = if failure.stats.success_count > 0 {
                        "partial"
                    } else {
                        "failed"
                    };
                    repo.finish_source_run(
                        run_id,
                        status,
                        failure.stats.request_count,
                        failure.stats.success_count,
                        Some(failure.category),
                        Some(&failure.message),
                    )
                    .map_err(err)?;
                    Err(format!(
                        "Steam candidate collection {} after {} requests and {} ingested rows: {}",
                        failure.category,
                        failure.stats.request_count,
                        failure.stats.success_count,
                        failure.message
                    ))
                }
            }
        }
        "backup" => {
            let db_path = required_path(args.next(), "--db path")?;
            let out_path = required_path(args.next(), "--out path")?;
            let db = Database::open(&db_path).map_err(err)?;
            db.assert_ready().map_err(err)?;
            let repo = Repository::new(db);
            repo.backup_to(&out_path).map_err(err)?;
            println!("backed up {} -> {}", db_path.display(), out_path.display());
            Ok(())
        }
        "restore" => {
            let backup_path = required_path(args.next(), "--from path")?;
            let dest_path = required_path(args.next(), "--to path")?;
            let now = SystemClock.now_ms();
            let repo = Repository::restore_backup(&backup_path, &dest_path, now).map_err(err)?;
            repo.assert_ready().map_err(err)?;
            println!(
                "restored {} -> {} (schema ok)",
                backup_path.display(),
                dest_path.display()
            );
            Ok(())
        }
        "help" | "-h" | "--help" => {
            println!("{}", usage());
            Ok(())
        }
        other => Err(format!("unknown command '{other}'\n{}", usage())),
    }
}

fn required_path(arg: Option<String>, label: &str) -> Result<PathBuf, String> {
    let value = arg.ok_or_else(|| format!("missing {label}"))?;
    // allow either bare path or --db path form
    if value.starts_with("--") {
        return Err(format!("expected path for {label}, got flag {value}"));
    }
    Ok(PathBuf::from(value))
}

fn optional_target(arg: Option<String>) -> Result<u32, String> {
    let target = match arg {
        Some(value) => value
            .parse::<u32>()
            .map_err(|_| format!("invalid candidate target: {value}"))?,
        None => STORE_SEARCH_TARGET_DEFAULT,
    };
    if !(1..=STORE_SEARCH_TARGET_MAX).contains(&target) {
        return Err(format!(
            "candidate target must be between 1 and {STORE_SEARCH_TARGET_MAX}"
        ));
    }
    Ok(target)
}

fn audit_m3(repo: &Repository, db_path: &std::path::Path) -> Result<(), String> {
    let active = repo.active_algorithm_config().map_err(err)?;
    let coverage = repo.m3_catalog_coverage().map_err(err)?;
    println!("path={}", db_path.display());
    println!("algorithm_version={}", active.version);
    println!(
        "normalized_multiplayer_candidates={}",
        coverage.normalized_multiplayer_candidates
    );
    println!(
        "category_evidence_candidates={}",
        coverage.category_evidence_candidates
    );
    println!(
        "recommendation_ready_profiles={}",
        coverage.recommendation_ready_profiles
    );
    println!(
        "trusted_familiar_profiles={}",
        coverage.trusted_familiar_profiles
    );
    println!("with_platforms={}", coverage.with_platforms);
    println!("with_languages={}", coverage.with_languages);
    println!("with_typical_session={}", coverage.with_typical_session);
    println!("with_price={}", coverage.with_price);
    println!("with_reviews={}", coverage.with_reviews);
    println!("with_ccu={}", coverage.with_ccu);
    if coverage.normalized_multiplayer_candidates < i64::from(STORE_SEARCH_TARGET_DEFAULT) {
        return Err(format!(
            "M3 catalog gate failed: {} normalized multiplayer candidates, need at least {}",
            coverage.normalized_multiplayer_candidates, STORE_SEARCH_TARGET_DEFAULT
        ));
    }
    println!("m3_catalog_gate=ok");
    Ok(())
}

fn collect_steam_candidates(
    repo: &Repository,
    target: u32,
) -> Result<CollectionStats, CollectionError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|error| CollectionError {
            category: "network",
            message: error.to_string(),
            stats: CollectionStats::default(),
        })?;
    let mut stats = CollectionStats::default();
    let coverage = repo
        .m3_catalog_coverage()
        .map_err(|error| CollectionError {
            category: "storage",
            message: error.to_string(),
            stats: CollectionStats::default(),
        })?;
    if coverage.normalized_multiplayer_candidates >= i64::from(target) {
        return Ok(stats);
    }

    let stored_cursor = repo
        .source_cursor(STORE_SEARCH_CURSOR_KEY)
        .map_err(|error| CollectionError {
            category: "storage",
            message: error.to_string(),
            stats: CollectionStats::default(),
        })?
        .map(|value| serde_json::from_str::<StoreSearchCursor>(&value))
        .transpose()
        .map_err(|error| CollectionError {
            category: "invalid_payload",
            message: format!("invalid stored cursor: {error}"),
            stats: CollectionStats::default(),
        })?;
    let mut start = stored_cursor
        .as_ref()
        .filter(|cursor| !cursor.complete)
        .map_or(0, |cursor| cursor.next_start);

    loop {
        let current = repo
            .m3_catalog_coverage()
            .map_err(|error| CollectionError {
                category: "storage",
                message: error.to_string(),
                stats,
            })?;
        if current.normalized_multiplayer_candidates >= i64::from(target) {
            return Ok(stats);
        }

        let request = StoreSearchRequest::new(start, 100).map_err(|error| CollectionError {
            category: source_error_category(&error),
            message: error.to_string(),
            stats,
        })?;
        let page = fetch_store_search_page_with_retry(&client, &request, &mut stats)?;
        let ingested = repo
            .ingest_store_search_page(&page)
            .map_err(|error| CollectionError {
                category: "storage",
                message: error.to_string(),
                stats,
            })?;
        stats.success_count = stats.success_count.saturating_add(ingested as i64);

        let next_start = page.next_start();
        let cursor = StoreSearchCursor {
            next_start,
            total_count: Some(page.total_count),
            target,
            complete: page.is_complete(),
        };
        repo.save_source_cursor(
            STORE_SEARCH_CURSOR_KEY,
            STORE_SEARCH_SOURCE_NAME,
            &serde_json::to_value(&cursor).map_err(|error| CollectionError {
                category: "invalid_payload",
                message: error.to_string(),
                stats,
            })?,
        )
        .map_err(|error| CollectionError {
            category: "storage",
            message: error.to_string(),
            stats,
        })?;

        let current = repo
            .m3_catalog_coverage()
            .map_err(|error| CollectionError {
                category: "storage",
                message: error.to_string(),
                stats,
            })?;
        println!(
            "progress candidates={} target={} page_start={} rows={}",
            current.normalized_multiplayer_candidates, target, page.start, page.result_count
        );
        if current.normalized_multiplayer_candidates >= i64::from(target) {
            return Ok(stats);
        }
        if page.is_complete() {
            return Err(CollectionError {
                category: "insufficient_results",
                message: format!(
                    "Steam search exhausted at {} candidates before target {target}",
                    current.normalized_multiplayer_candidates
                ),
                stats,
            });
        }
        start = next_start;
        thread::sleep(Duration::from_millis(1_100));
    }
}

fn fetch_store_search_page_with_retry(
    client: &reqwest::blocking::Client,
    request: &StoreSearchRequest,
    stats: &mut CollectionStats,
) -> Result<StoreSearchPage, CollectionError> {
    let mut last_error = None;
    for attempt in 0..3_u64 {
        stats.request_count = stats.request_count.saturating_add(1);
        match fetch_store_search_page(client, request) {
            Ok(page) => return Ok(page),
            Err(error) => {
                let retryable = error.is_retryable();
                let category = source_error_category(&error);
                let message = error.to_string();
                last_error = Some((category, message));
                if !retryable || attempt == 2 {
                    break;
                }
                thread::sleep(Duration::from_secs(1_u64 << attempt));
            }
        }
    }
    let (category, message) = last_error.unwrap_or(("network", "unknown fetch failure".into()));
    Err(CollectionError {
        category,
        message,
        stats: CollectionStats {
            request_count: stats.request_count,
            success_count: stats.success_count,
        },
    })
}

fn fetch_store_search_page(
    client: &reqwest::blocking::Client,
    request: &StoreSearchRequest,
) -> Result<StoreSearchPage, SourceError> {
    let url = format!("{STEAM_STORE_HOST}{}", request.path_and_query());
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|error| SourceError::Temporary {
            message: error.to_string(),
        })?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let mut body = Vec::new();
    response
        .take((STORE_SEARCH_RESPONSE_MAX_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| SourceError::Temporary {
            message: error.to_string(),
        })?;
    let raw = RawResponse::validate(status, body, content_type, STORE_SEARCH_RESPONSE_MAX_BYTES)?;
    parse_store_search_page(request, &raw)
}

fn source_error_category(error: &SourceError) -> &'static str {
    match error {
        SourceError::RateLimited { .. } => "rate_limited",
        SourceError::HttpStatus { status: 401 | 403 } => "auth",
        SourceError::NotFound { .. } => "not_found",
        SourceError::JsonParse { .. }
        | SourceError::InvalidStructure { .. }
        | SourceError::InvalidUtf8 => "parse_changed",
        SourceError::Config { .. } | SourceError::Permanent { .. } => "invalid_payload",
        SourceError::ResponseTooLarge { .. } => "response_too_large",
        SourceError::HttpStatus { .. } | SourceError::Temporary { .. } => "network",
    }
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn usage() -> &'static str {
    "mpgs-dbtool <command> [args]\n\n\
     Commands:\n\
       migrate <db-path>\n\
       integrity <db-path>\n\
       m3-audit <db-path>\n\
       collect-steam-candidates <db-path> [target, default 2000]\n\
       backup <db-path> <backup-path>\n\
       restore <backup-path> <dest-db-path>\n"
}
