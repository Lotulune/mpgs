use serde::{Deserialize, Serialize};

/// Durable GetAppList pagination / incremental checkpoint.
///
/// Persist this value between runs to resume pagination or apply
/// `if_modified_since` incremental pulls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppListCursor {
    /// Last successfully processed `appid` for `last_appid` pagination.
    pub last_appid: u32,
    /// Unix seconds watermark passed as `if_modified_since` on the next full pass.
    pub if_modified_since: u32,
    /// Highest `last_modified` observed in the current completed pass.
    pub high_water_last_modified: u32,
    /// True when the previous response indicated more pages remain.
    pub have_more_results: bool,
    /// Adapter version that produced this cursor.
    pub adapter_version: String,
}

impl AppListCursor {
    pub fn new_pass(if_modified_since: u32, adapter_version: impl Into<String>) -> Self {
        Self {
            last_appid: 0,
            if_modified_since,
            high_water_last_modified: if_modified_since,
            have_more_results: true,
            adapter_version: adapter_version.into(),
        }
    }

    /// Advance after a successful page parse.
    pub fn advance_page(
        &mut self,
        page_last_appid: u32,
        page_max_last_modified: u32,
        have_more_results: bool,
    ) {
        self.last_appid = page_last_appid;
        self.have_more_results = have_more_results;
        self.high_water_last_modified = self.high_water_last_modified.max(page_max_last_modified);
    }

    /// Complete a full directory pass and prepare the next incremental window.
    pub fn complete_pass(&mut self) {
        self.last_appid = 0;
        self.have_more_results = false;
        self.if_modified_since = self.high_water_last_modified;
    }

    pub fn is_in_progress(&self) -> bool {
        self.have_more_results || self.last_appid > 0
    }
}

#[cfg(test)]
mod tests {
    use super::AppListCursor;

    #[test]
    fn resume_and_complete_incremental_pass() {
        let mut cursor = AppListCursor::new_pass(1_700_000_000, "app-list-0.1.0");
        cursor.advance_page(100, 1_700_000_100, true);
        assert!(cursor.is_in_progress());
        assert_eq!(cursor.last_appid, 100);

        cursor.advance_page(200, 1_700_000_200, false);
        cursor.complete_pass();
        assert_eq!(cursor.last_appid, 0);
        assert_eq!(cursor.if_modified_since, 1_700_000_200);
        assert!(!cursor.have_more_results);
    }
}
