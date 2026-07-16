//! Steam Store `appdetails` volatile adapter spike (calendar / Demo relations).
//!
//! This endpoint is **not** a documented stable Web API contract. It is isolated
//! here so M2 can swap or disable it without touching official adapters.

use serde::Deserialize;
use serde_json::Value;

use crate::error::SourceError;
use crate::proposal::{
    AppRelationProposal, AppTypeProposal, RelationTypeProposal, ReleaseStateProposal,
    SourceStability, StoreDetailsProposal, StorePriceProposal,
};
use crate::raw::RawResponse;

pub const ADAPTER_VERSION: &str = "store-appdetails-0.2.0";
pub const SOURCE_NAME: &str = "steam_store_appdetails";
/// Default storefront region for price snapshots (ISO country, lower-case query).
pub const DEFAULT_STORE_COUNTRY: &str = "cn";
pub const DEFAULT_STORE_LANGUAGE: &str = "schinese";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreDetailsRequest {
    pub app_id: u32,
    /// Two-letter Steam store country code controlling price_overview currency.
    pub country_code: String,
    /// Steam locale used for localized store text.
    pub language: String,
}

impl StoreDetailsRequest {
    pub fn new(app_id: u32) -> Self {
        Self {
            app_id,
            country_code: DEFAULT_STORE_COUNTRY.into(),
            language: DEFAULT_STORE_LANGUAGE.into(),
        }
    }

    pub fn with_locale(
        app_id: u32,
        country_code: impl Into<String>,
        language: impl Into<String>,
    ) -> Result<Self, SourceError> {
        let country_code = country_code.into().trim().to_ascii_lowercase();
        let language = language.into().trim().to_ascii_lowercase();
        if country_code.len() != 2 || !country_code.bytes().all(|b| b.is_ascii_alphabetic()) {
            return Err(SourceError::Config {
                message: "store country must be a two-letter ISO code".into(),
            });
        }
        if language.is_empty()
            || !language
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        {
            return Err(SourceError::Config {
                message: "store language must be a non-empty Steam locale".into(),
            });
        }
        Ok(Self {
            app_id,
            country_code,
            language,
        })
    }

    pub fn path_and_query(&self) -> String {
        format!(
            "/api/appdetails?appids={}&l={}&cc={}",
            self.app_id, self.language, self.country_code
        )
    }
}

#[derive(Debug, Deserialize)]
struct AppDetailsNode {
    success: bool,
    #[serde(default)]
    data: Option<AppDetailsData>,
}

#[derive(Debug, Deserialize)]
struct AppDetailsData {
    #[serde(default)]
    #[serde(rename = "type")]
    app_type: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    steam_appid: Option<u32>,
    #[serde(default)]
    is_free: Option<bool>,
    #[serde(default)]
    platforms: Option<PlatformsDto>,
    #[serde(default)]
    supported_languages: Option<String>,
    #[serde(default)]
    short_description: Option<String>,
    #[serde(default)]
    developers: Option<Vec<String>>,
    #[serde(default)]
    publishers: Option<Vec<String>>,
    #[serde(default)]
    categories: Option<Vec<CategoryDto>>,
    #[serde(default)]
    genres: Option<Vec<GenreDto>>,
    #[serde(default)]
    release_date: Option<ReleaseDateDto>,
    #[serde(default)]
    demos: Option<Vec<DemoDto>>,
    #[serde(default)]
    fullgame: Option<FullGameDto>,
    #[serde(default)]
    price_overview: Option<PriceOverviewDto>,
    #[serde(default)]
    packages: Option<Vec<i64>>,
}

#[derive(Debug, Deserialize)]
struct PriceOverviewDto {
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    initial: Option<i64>,
    #[serde(default)]
    #[serde(rename = "final")]
    final_price: Option<i64>,
    #[serde(default)]
    discount_percent: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct PlatformsDto {
    #[serde(default)]
    windows: bool,
    #[serde(default)]
    mac: bool,
    #[serde(default)]
    linux: bool,
}

impl PlatformsDto {
    fn normalized(self) -> Vec<String> {
        [
            ("windows", self.windows),
            ("mac", self.mac),
            ("linux", self.linux),
        ]
        .into_iter()
        .filter(|(_, supported)| *supported)
        .map(|(name, _)| name.to_owned())
        .collect()
    }
}

#[derive(Debug, Deserialize)]
struct CategoryDto {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenreDto {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseDateDto {
    #[serde(default)]
    coming_soon: Option<bool>,
    #[serde(default)]
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DemoDto {
    #[serde(default)]
    appid: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FullGameDto {
    #[serde(default)]
    appid: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoreDetailsParseResult {
    pub details: StoreDetailsProposal,
    pub relations: Vec<AppRelationProposal>,
}

pub fn parse_store_details(
    request: &StoreDetailsRequest,
    raw: &RawResponse,
) -> Result<StoreDetailsParseResult, SourceError> {
    let root: Value = raw.parse_json()?;
    let key = request.app_id.to_string();
    let node_value = root
        .get(&key)
        .ok_or_else(|| SourceError::invalid_structure(format!("missing top-level key {key}")))?;

    let node: AppDetailsNode =
        serde_json::from_value(node_value.clone()).map_err(SourceError::json_parse)?;

    if !node.success {
        return Err(SourceError::NotFound { entity_key: key });
    }

    let data = node
        .data
        .ok_or_else(|| SourceError::invalid_structure("success=true but data object is missing"))?;

    if data
        .steam_appid
        .is_some_and(|response_id| response_id != request.app_id)
    {
        return Err(SourceError::invalid_structure(format!(
            "steam_appid does not match requested app {}",
            request.app_id
        )));
    }
    let app_id = request.app_id;
    let categories: Vec<String> = data
        .categories
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| c.description)
        .collect();
    let genres: Vec<String> = data
        .genres
        .unwrap_or_default()
        .into_iter()
        .filter_map(|g| g.description)
        .collect();

    let multiplayer_category_hints = categories
        .iter()
        .filter(|c| is_multiplayer_hint(c))
        .cloned()
        .collect();

    let release_date_observed = data.release_date.is_some();
    let coming_soon = data.release_date.as_ref().and_then(|r| r.coming_soon);
    let release_date_raw = data
        .release_date
        .as_ref()
        .and_then(|r| r.date.clone())
        .filter(|d| !d.trim().is_empty());
    let (release_date, release_date_precision) =
        normalize_release_date(release_date_raw.as_deref());

    let release_state = match coming_soon {
        Some(true) => ReleaseStateProposal::ComingSoon,
        Some(false) => ReleaseStateProposal::Released,
        None => ReleaseStateProposal::Unknown,
    };

    let demo_app_ids: Vec<u32> = data
        .demos
        .unwrap_or_default()
        .into_iter()
        .filter_map(|d| d.appid)
        .filter(|id| *id != 0)
        .collect();

    let fullgame_app_id = data
        .fullgame
        .and_then(|f| f.appid)
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|id| *id != 0);

    let app_type = data
        .app_type
        .as_deref()
        .map(AppTypeProposal::from_steam_type)
        .unwrap_or(AppTypeProposal::Unknown);
    let platforms = data.platforms.map(PlatformsDto::normalized);
    let supported_languages = data
        .supported_languages
        .as_deref()
        .map(normalize_supported_languages);
    let package_id = data
        .packages
        .as_ref()
        .and_then(|packages| packages.first())
        .map(|id| id.to_string());
    let price = normalize_price(
        data.is_free,
        data.price_overview,
        &request.country_code,
        package_id,
    );

    let details = StoreDetailsProposal {
        app_id,
        name: data.name,
        app_type,
        release_state,
        release_date_raw,
        release_date,
        release_date_precision,
        release_date_observed,
        is_free: data.is_free,
        platforms,
        supported_languages,
        price,
        coming_soon,
        categories,
        genres,
        developers: data.developers.unwrap_or_default(),
        publishers: data.publishers.unwrap_or_default(),
        short_description: data.short_description,
        demo_app_ids: demo_app_ids.clone(),
        fullgame_app_id,
        multiplayer_category_hints,
        content_hash: raw.content_hash.clone(),
        source: SOURCE_NAME,
        stability: SourceStability::ApprovedVolatile,
        adapter_version: ADAPTER_VERSION,
    };

    let mut relations = Vec::new();
    for demo_id in demo_app_ids {
        relations.push(AppRelationProposal {
            source_app_id: demo_id,
            target_app_id: app_id,
            relation_type: RelationTypeProposal::DemoOf,
            confidence: 0.7,
            stability: SourceStability::ApprovedVolatile,
            adapter_version: ADAPTER_VERSION,
        });
    }
    if let Some(full_id) = fullgame_app_id {
        let relation_type = match details.app_type {
            AppTypeProposal::Playtest => RelationTypeProposal::PlaytestOf,
            _ => RelationTypeProposal::DemoOf,
        };
        relations.push(AppRelationProposal {
            source_app_id: app_id,
            target_app_id: full_id,
            relation_type,
            confidence: 0.75,
            stability: SourceStability::ApprovedVolatile,
            adapter_version: ADAPTER_VERSION,
        });
    }

    Ok(StoreDetailsParseResult { details, relations })
}

fn normalize_price(
    is_free: Option<bool>,
    overview: Option<PriceOverviewDto>,
    country_code: &str,
    package_id: Option<String>,
) -> Option<StorePriceProposal> {
    let country = country_code.trim().to_ascii_uppercase();
    if country.is_empty() {
        return None;
    }
    if is_free == Some(true) {
        let currency = currency_for_country(&country)?;
        return Some(StorePriceProposal {
            country_code: country,
            currency: currency.into(),
            initial_price_minor: Some(0),
            final_price_minor: Some(0),
            discount_percent: Some(0),
            is_purchasable: Some(true),
            package_id,
        });
    }
    if let Some(overview) = overview {
        let currency = overview
            .currency
            .map(|value| value.trim().to_ascii_uppercase())
            .filter(|value| !value.is_empty())?;
        return Some(StorePriceProposal {
            country_code: country,
            currency,
            initial_price_minor: overview.initial,
            final_price_minor: overview.final_price,
            discount_percent: overview.discount_percent,
            is_purchasable: Some(true),
            package_id,
        });
    }
    // No currency was returned. Persisting a guessed USD marker here would make
    // regional budget filters silently consume invalid data.
    None
}

fn currency_for_country(country: &str) -> Option<&'static str> {
    match country {
        "CN" => Some("CNY"),
        "US" => Some("USD"),
        "GB" => Some("GBP"),
        "JP" => Some("JPY"),
        "KR" => Some("KRW"),
        "CA" => Some("CAD"),
        "AU" => Some("AUD"),
        "DE" | "ES" | "FR" | "IT" | "NL" | "PT" => Some("EUR"),
        _ => None,
    }
}

fn normalize_supported_languages(raw: &str) -> Vec<String> {
    const LANGUAGES: &[(&str, &str)] = &[
        ("Simplified Chinese", "schinese"),
        ("Traditional Chinese", "tchinese"),
        ("Portuguese - Brazil", "brazilian"),
        ("Brazilian Portuguese", "brazilian"),
        ("Spanish - Latin America", "latam"),
        ("Arabic", "arabic"),
        ("Bulgarian", "bulgarian"),
        ("Czech", "czech"),
        ("Danish", "danish"),
        ("Dutch", "dutch"),
        ("English", "english"),
        ("Finnish", "finnish"),
        ("French", "french"),
        ("German", "german"),
        ("Greek", "greek"),
        ("Hungarian", "hungarian"),
        ("Indonesian", "indonesian"),
        ("Italian", "italian"),
        ("Japanese", "japanese"),
        ("Korean", "koreana"),
        ("Norwegian", "norwegian"),
        ("Polish", "polish"),
        ("Portuguese", "portuguese"),
        ("Romanian", "romanian"),
        ("Russian", "russian"),
        ("Spanish - Spain", "spanish"),
        ("Spanish", "spanish"),
        ("Swedish", "swedish"),
        ("Thai", "thai"),
        ("Turkish", "turkish"),
        ("Ukrainian", "ukrainian"),
        ("Vietnamese", "vietnamese"),
    ];
    let lower = raw.to_ascii_lowercase();
    let mut found = Vec::new();
    for (label, code) in LANGUAGES {
        if lower.contains(&label.to_ascii_lowercase()) && !found.iter().any(|item| item == code) {
            found.push((*code).to_owned());
        }
    }
    found
}

fn normalize_release_date(raw: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return (None, None);
    };

    if let Some((year, month, day)) = parse_iso_day(raw) {
        return (
            Some(format!("{year:04}-{month:02}-{day:02}")),
            Some("day".into()),
        );
    }

    let cleaned = raw.replace(',', " ");
    let parts: Vec<_> = cleaned.split_whitespace().collect();
    if parts.len() == 3 {
        let parsed = if let (Ok(day), Some(month), Ok(year)) = (
            parts[0].parse::<u32>(),
            month_number(parts[1]),
            parts[2].parse::<i32>(),
        ) {
            Some((year, month, day))
        } else if let (Some(month), Ok(day), Ok(year)) = (
            month_number(parts[0]),
            parts[1].parse::<u32>(),
            parts[2].parse::<i32>(),
        ) {
            Some((year, month, day))
        } else {
            None
        };
        if let Some((year, month, day)) = parsed.filter(|&(y, m, d)| valid_day(y, m, d)) {
            return (
                Some(format!("{year:04}-{month:02}-{day:02}")),
                Some("day".into()),
            );
        }
    }

    if parts.len() == 2 {
        let quarter = parts[0]
            .strip_prefix('Q')
            .or_else(|| parts[0].strip_prefix('q'))
            .and_then(|value| value.parse::<u8>().ok());
        if quarter.is_some_and(|value| (1..=4).contains(&value)) && parts[1].parse::<i32>().is_ok()
        {
            return (None, Some("quarter".into()));
        }
        if month_number(parts[0]).is_some() && parts[1].parse::<i32>().is_ok() {
            return (None, Some("month".into()));
        }
    }

    if raw.parse::<i32>().is_ok() {
        return (None, Some("year".into()));
    }
    (None, Some("tba".into()))
}

fn parse_iso_day(value: &str) -> Option<(i32, u32, u32)> {
    if value.len() != 10
        || !value.is_ascii()
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return None;
    }
    let year = value[0..4].parse().ok()?;
    let month = value[5..7].parse().ok()?;
    let day = value[8..10].parse().ok()?;
    valid_day(year, month, day).then_some((year, month, day))
}

fn month_number(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn valid_day(year: i32, month: u32, day: u32) -> bool {
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn is_multiplayer_hint(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    lower.contains("multi")
        || lower.contains("co-op")
        || lower.contains("coop")
        || lower.contains("pvp")
        || lower.contains("online")
        || lower.contains("mmo")
        || lower.contains("lan")
}

/// Static feasibility summary used by docs and runtime diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoreAdapterFeasibility {
    pub endpoint: &'static str,
    pub stability: SourceStability,
    pub supports_release_calendar: bool,
    pub supports_demo_relation: bool,
    pub requires_web_api_key: bool,
    pub recommended_fallback: &'static str,
}

pub const STORE_APPDETAILS_FEASIBILITY: StoreAdapterFeasibility = StoreAdapterFeasibility {
    endpoint: "https://store.steampowered.com/api/appdetails",
    stability: SourceStability::ApprovedVolatile,
    supports_release_calendar: true,
    supports_demo_relation: true,
    requires_web_api_key: false,
    recommended_fallback: "human curation + release_events table when structure changes",
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raw::RawResponse;

    fn fixture(name: &str) -> RawResponse {
        let body = match name {
            "game" => include_bytes!("../fixtures/store_appdetails_game.json").to_vec(),
            "demo" => include_bytes!("../fixtures/store_appdetails_demo.json").to_vec(),
            "coming_soon" => {
                include_bytes!("../fixtures/store_appdetails_coming_soon.json").to_vec()
            }
            "fail" => include_bytes!("../fixtures/store_appdetails_fail.json").to_vec(),
            other => panic!("unknown fixture {other}"),
        };
        RawResponse::validate(200, body, Some("application/json".into()), 1024 * 1024).unwrap()
    }

    #[test]
    fn parses_released_game_with_demo_relation() {
        let request = StoreDetailsRequest::with_locale(892970, "US", "english").unwrap();
        let result = parse_store_details(&request, &fixture("game")).unwrap();
        assert_eq!(result.details.app_id, 892970);
        assert_eq!(result.details.release_state, ReleaseStateProposal::Released);
        assert_eq!(result.details.release_date.as_deref(), Some("2021-02-02"));
        assert_eq!(
            result.details.release_date_precision.as_deref(),
            Some("day")
        );
        assert_eq!(result.details.stability, SourceStability::ApprovedVolatile);
        assert!(!result.details.multiplayer_category_hints.is_empty());
        let price = result.details.price.expect("price_overview");
        assert_eq!(price.country_code, "US");
        assert_eq!(price.currency, "USD");
        assert_eq!(price.initial_price_minor, Some(1999));
        assert_eq!(price.final_price_minor, Some(999));
        assert_eq!(price.discount_percent, Some(50));
        assert_eq!(price.package_id.as_deref(), Some("123456"));
        assert!(result.relations.iter().any(|r| {
            matches!(r.relation_type, RelationTypeProposal::DemoOf) && r.target_app_id == 892970
        }));
    }

    #[test]
    fn free_game_synthesizes_zero_price_snapshot() {
        let request = StoreDetailsRequest::new(570);
        let raw = RawResponse::validate(
            200,
            br#"{"570":{"success":true,"data":{"type":"game","name":"Dota 2","steam_appid":570,"is_free":true}}}"#
                .to_vec(),
            Some("application/json".into()),
            1024,
        )
        .unwrap();
        let result = parse_store_details(&request, &raw).unwrap();
        let price = result.details.price.expect("free price");
        assert_eq!(price.final_price_minor, Some(0));
        assert_eq!(price.currency, "CNY");
        assert_eq!(price.is_purchasable, Some(true));
    }

    #[test]
    fn missing_price_overview_does_not_invent_a_currency() {
        let request = StoreDetailsRequest::new(42);
        let raw = RawResponse::validate(
            200,
            br#"{"42":{"success":true,"data":{"type":"game","name":"Soon","steam_appid":42,"is_free":false}}}"#
                .to_vec(),
            Some("application/json".into()),
            1024,
        )
        .unwrap();
        let result = parse_store_details(&request, &raw).unwrap();
        assert_eq!(result.details.price, None);
    }

    #[test]
    fn request_includes_country_for_price_region() {
        let request = StoreDetailsRequest::new(1);
        assert!(request.path_and_query().contains("cc=cn"));
    }

    #[test]
    fn parses_demo_fullgame_link() {
        let request = StoreDetailsRequest::new(1_888_930);
        let result = parse_store_details(&request, &fixture("demo")).unwrap();
        assert_eq!(result.details.app_type, AppTypeProposal::Demo);
        assert_eq!(result.details.fullgame_app_id, Some(892970));
        assert!(result.relations.iter().any(|r| {
            r.source_app_id == 1_888_930
                && r.target_app_id == 892970
                && matches!(r.relation_type, RelationTypeProposal::DemoOf)
        }));
    }

    #[test]
    fn parses_coming_soon_calendar_fields() {
        let request = StoreDetailsRequest::new(2_500_000);
        let result = parse_store_details(&request, &fixture("coming_soon")).unwrap();
        assert_eq!(
            result.details.release_state,
            ReleaseStateProposal::ComingSoon
        );
        assert_eq!(result.details.coming_soon, Some(true));
        assert!(result.details.release_date_raw.is_some());
        assert_eq!(result.details.release_date, None);
        assert_eq!(
            result.details.release_date_precision.as_deref(),
            Some("quarter")
        );
    }

    #[test]
    fn unsuccessful_appdetails_is_not_found() {
        let request = StoreDetailsRequest::new(1);
        let err = parse_store_details(&request, &fixture("fail")).unwrap_err();
        assert!(matches!(err, SourceError::NotFound { .. }));
    }

    #[test]
    fn feasibility_marks_store_as_volatile() {
        assert_eq!(
            STORE_APPDETAILS_FEASIBILITY.stability,
            SourceStability::ApprovedVolatile
        );
        const {
            assert!(STORE_APPDETAILS_FEASIBILITY.supports_demo_relation);
            assert!(!STORE_APPDETAILS_FEASIBILITY.requires_web_api_key);
        }
    }

    #[test]
    fn request_uses_china_region_matching_default_cny_preferences() {
        assert_eq!(
            StoreDetailsRequest::new(42).path_and_query(),
            "/api/appdetails?appids=42&l=schinese&cc=cn"
        );
    }

    #[test]
    fn request_locale_is_configurable_and_validated() {
        let request = StoreDetailsRequest::with_locale(42, "US", "english").unwrap();
        assert_eq!(
            request.path_and_query(),
            "/api/appdetails?appids=42&l=english&cc=us"
        );
        assert!(StoreDetailsRequest::with_locale(42, "USA", "english").is_err());
        assert!(StoreDetailsRequest::with_locale(42, "US", "../bad").is_err());
    }

    #[test]
    fn response_app_id_must_match_request() {
        let raw = RawResponse::validate(
            200,
            br#"{"42":{"success":true,"data":{"steam_appid":43,"type":"game"}}}"#.to_vec(),
            Some("application/json".into()),
            1024,
        )
        .unwrap();
        let error = parse_store_details(&StoreDetailsRequest::new(42), &raw).unwrap_err();
        assert!(matches!(error, SourceError::InvalidStructure { .. }));
    }

    #[test]
    fn parses_structured_platforms_and_normalizes_languages() {
        let raw = RawResponse::validate(
            200,
            br#"{"42":{"success":true,"data":{"steam_appid":42,"type":"game","platforms":{"windows":true,"mac":false,"linux":true},"supported_languages":"English, Simplified Chinese<strong>*</strong>, Japanese"}}}"#.to_vec(),
            Some("application/json".into()),
            4096,
        )
        .unwrap();
        let result = parse_store_details(&StoreDetailsRequest::new(42), &raw).unwrap();
        assert_eq!(
            result.details.platforms,
            Some(vec!["windows".into(), "linux".into()])
        );
        assert_eq!(
            result.details.supported_languages,
            Some(vec!["schinese".into(), "english".into(), "japanese".into()])
        );
    }

    #[test]
    fn normalizes_macos_to_the_client_platform_identifier() {
        let raw = RawResponse::validate(
            200,
            br#"{"42":{"success":true,"data":{"steam_appid":42,"type":"game","platforms":{"windows":false,"mac":true,"linux":false}}}}"#.to_vec(),
            Some("application/json".into()),
            4096,
        )
        .unwrap();
        let result = parse_store_details(&StoreDetailsRequest::new(42), &raw).unwrap();
        assert_eq!(result.details.platforms, Some(vec!["mac".into()]));
    }
}
