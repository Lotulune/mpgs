#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use mpgs_storage::{Clock, Database, Repository, SystemClock};

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

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn usage() -> &'static str {
    "mpgs-dbtool <command> [args]\n\n\
     Commands:\n\
       migrate <db-path>\n\
       integrity <db-path>\n\
       backup <db-path> <backup-path>\n\
       restore <backup-path> <dest-db-path>\n"
}
