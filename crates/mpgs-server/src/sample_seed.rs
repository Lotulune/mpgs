use crate::db;
use anyhow::{anyhow, Result};
use base64::Engine;
use mpgs_core::analysis::build_rule_report;
use mpgs_core::models::{GameAnalysisReport, GameCard, ReviewSnippet, StoreReleaseState};
use mpgs_core::recommendation::DemoStatus;
use mpgs_core::steam_mapping::{build_discovered_game_card, SteamAppListItem, SteamGameSnapshot};
use serde::Serialize;
use sqlx_postgres::PgPool;

pub const SAMPLE_PUBLIC_CATALOG_SEED_CONFIRM_ENV: &str = "MPGS_ALLOW_SAMPLE_CATALOG_SEED";

const SAMPLE_TODAY: &str = "2026-06-12";
const SAMPLE_GENERATED_AT: &str = "2026-06-12T00:00:00Z";

#[derive(Debug, Clone)]
pub struct SamplePublicCatalogGame {
    pub game: GameCard,
    pub report: GameAnalysisReport,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SamplePublicCatalogSeedSummary {
    pub sample_count: usize,
    pub public_games: usize,
    pub appids: Vec<u32>,
}

pub fn sample_public_catalog_games() -> Result<Vec<SamplePublicCatalogGame>> {
    sample_specs()
        .into_iter()
        .map(build_sample_public_catalog_game)
        .collect()
}

pub async fn seed_sample_public_catalog(pool: &PgPool) -> Result<SamplePublicCatalogSeedSummary> {
    let samples = sample_public_catalog_games()?;
    let mut public_games = 0;
    let mut appids = Vec::with_capacity(samples.len());

    for sample in &samples {
        let result = db::upsert_public_catalog_game(
            pool,
            &sample.game,
            &sample.report,
            "accepted",
            "public",
        )
        .await?;

        if result.review_status == "accepted" && result.visibility == "public" {
            public_games += 1;
        }
        appids.push(result.appid);
    }

    Ok(SamplePublicCatalogSeedSummary {
        sample_count: samples.len(),
        public_games,
        appids,
    })
}

fn build_sample_public_catalog_game(spec: SampleSpec) -> Result<SamplePublicCatalogGame> {
    let app = SteamAppListItem {
        appid: spec.appid,
        name: spec.name.to_string(),
    };
    let game = build_discovered_game_card(&app, spec.snapshot(), SAMPLE_TODAY)
        .ok_or_else(|| anyhow!("sample game {} did not pass discovery mapping", spec.appid))?;
    let report = build_rule_report(&game, SAMPLE_GENERATED_AT.to_string())?;

    Ok(SamplePublicCatalogGame { game, report })
}

#[derive(Debug, Clone, Copy)]
struct SampleSpec {
    appid: u32,
    name: &'static str,
    short_description: &'static str,
    release_date: &'static str,
    release_date_text: &'static str,
    price_text: &'static str,
    positive_review_pct: f64,
    total_reviews: u32,
    current_players: u32,
    tags: &'static [&'static str],
    multiplayer_modes: &'static [&'static str],
    review_quote: &'static str,
}

impl SampleSpec {
    fn snapshot(self) -> SteamGameSnapshot {
        SteamGameSnapshot {
            name: Some(self.name.to_string()),
            short_description: Some(self.short_description.to_string()),
            release_date: Some(self.release_date.to_string()),
            release_date_text: Some(self.release_date_text.to_string()),
            release_state: Some(StoreReleaseState::Released),
            demo_status: DemoStatus::ReleasedWithDemo,
            supported_languages: Some(vec![
                "English".to_string(),
                "Simplified Chinese".to_string(),
            ]),
            is_adult_content: Some(false),
            is_free: Some(false),
            price_text: Some(self.price_text.to_string()),
            discount_percent: Some(10),
            positive_review_pct: Some(self.positive_review_pct),
            total_reviews: Some(self.total_reviews),
            current_players: Some(self.current_players),
            capsule_url: Some(sample_image_data_uri(self.name, self.appid, "#f6d365")),
            store_screenshot_urls: vec![sample_image_data_uri(self.name, self.appid, "#9fd3c7")],
            tags: self.tags.iter().map(|tag| (*tag).to_string()).collect(),
            multiplayer_modes: self
                .multiplayer_modes
                .iter()
                .map(|mode| (*mode).to_string())
                .collect(),
            review_snippets: vec![ReviewSnippet {
                voted_up: true,
                review: self.review_quote.to_string(),
                playtime_hours: Some(18.0),
            }],
        }
    }
}

fn sample_image_data_uri(label: &str, appid: u32, accent: &str) -> String {
    let label = escape_svg_text(label);
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="616" height="353" viewBox="0 0 616 353">
<rect width="616" height="353" fill="#fff8ec"/>
<rect x="28" y="28" width="560" height="297" rx="20" fill="{accent}"/>
<circle cx="94" cy="92" r="34" fill="#1f2937" opacity="0.16"/>
<circle cx="520" cy="263" r="46" fill="#1f2937" opacity="0.10"/>
<text x="308" y="167" text-anchor="middle" font-family="Arial, sans-serif" font-size="38" font-weight="700" fill="#1f2937">{label}</text>
<text x="308" y="212" text-anchor="middle" font-family="Arial, sans-serif" font-size="18" fill="#374151">MPGS sample catalog · {appid}</text>
</svg>"##
    );
    let encoded = base64::engine::general_purpose::STANDARD.encode(svg.as_bytes());

    format!("data:image/svg+xml;base64,{encoded}")
}

fn escape_svg_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sample_specs() -> Vec<SampleSpec> {
    vec![
        SampleSpec {
            appid: 920_001,
            name: "Harbor Havoc Co-op",
            short_description:
                "A compact four-player harbor defense game built around quick co-op sessions.",
            release_date: "2026-05-18",
            release_date_text: "May 18, 2026",
            price_text: "$14.99",
            positive_review_pct: 94.0,
            total_reviews: 8_400,
            current_players: 2_300,
            tags: &["Co-op", "Action", "Online Co-Op", "PvE"],
            multiplayer_modes: &["Online Co-op", "Multi-player"],
            review_quote: "Easy to teach, loud in the best way, and great for a weeknight squad.",
        },
        SampleSpec {
            appid: 920_002,
            name: "Astral Kitchen Crew",
            short_description:
                "A frantic kitchen routing game where friends cook across drifting space stations.",
            release_date: "2025-11-05",
            release_date_text: "Nov 5, 2025",
            price_text: "$19.99",
            positive_review_pct: 91.0,
            total_reviews: 42_000,
            current_players: 5_200,
            tags: &["Casual", "Co-op", "Cooking", "Family Friendly"],
            multiplayer_modes: &["Online Co-op", "Shared/Split Screen Co-op", "Multi-player"],
            review_quote:
                "Chaotic without becoming mean, and every stage gives the group a new joke.",
        },
        SampleSpec {
            appid: 920_003,
            name: "Mech Rally Rivals",
            short_description:
                "Team-based mech racing with short championships and readable combat roles.",
            release_date: "2024-09-12",
            release_date_text: "Sep 12, 2024",
            price_text: "$24.99",
            positive_review_pct: 88.0,
            total_reviews: 27_500,
            current_players: 3_100,
            tags: &["Racing", "Team-Based", "Robots", "Competitive"],
            multiplayer_modes: &["Online PvP", "Multi-player", "Co-op"],
            review_quote: "The handling is clean and the team abilities keep every race close.",
        },
        SampleSpec {
            appid: 920_004,
            name: "Deep Signal Salvage",
            short_description:
                "A two-to-four player salvage roguelite about reading signals under pressure.",
            release_date: "2026-02-02",
            release_date_text: "Feb 2, 2026",
            price_text: "$16.99",
            positive_review_pct: 96.0,
            total_reviews: 13_600,
            current_players: 1_700,
            tags: &["Roguelite", "Co-op", "Sci-fi", "Exploration"],
            multiplayer_modes: &["Online Co-op", "Multi-player"],
            review_quote: "Every run creates a tense little story and the roles actually matter.",
        },
    ]
}
