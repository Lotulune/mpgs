use std::collections::BTreeSet;

use mpgs_server::sample_seed::{
    sample_public_catalog_games, SAMPLE_PUBLIC_CATALOG_SEED_CONFIRM_ENV,
};

#[test]
fn sample_public_catalog_seed_builds_visible_games_with_reports() {
    let samples = sample_public_catalog_games().expect("sample public catalog should build");

    assert!(
        samples.len() >= 3,
        "sample catalog should be large enough for dashboard sections"
    );
    let appids: BTreeSet<_> = samples.iter().map(|sample| sample.game.appid).collect();
    assert_eq!(appids.len(), samples.len(), "sample appids must be unique");
    assert_eq!(
        SAMPLE_PUBLIC_CATALOG_SEED_CONFIRM_ENV,
        "MPGS_ALLOW_SAMPLE_CATALOG_SEED"
    );

    for sample in samples {
        assert!(!sample.game.name.trim().is_empty());
        assert!(
            sample.game.recommendation_score > 0.0,
            "sample game should be scoreable"
        );
        assert!(
            sample
                .game
                .capsule_url
                .starts_with("data:image/svg+xml;base64,"),
            "sample images should be inline and deterministic"
        );
        assert!(
            !sample.game.capsule_url.contains("example.test"),
            "sample images should not depend on unreachable placeholder hosts"
        );
        assert_eq!(sample.report.appid, sample.game.appid);
        assert!(
            sample.report.overview.contains(&sample.game.name)
                || !sample.report.overview.trim().is_empty()
        );
    }
}
