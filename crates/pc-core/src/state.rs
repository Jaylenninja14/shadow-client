//! Small UI/application state that is not a "setting": onboarding progress, selection,
//! benchmark results, recently viewed projects.

use serde::{Deserialize, Serialize};

use crate::paths::AppPaths;
use crate::performance::benchmark::BenchmarkResult;
use crate::util;
use crate::Result;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RecentProject {
    pub platform: String,
    pub id: String,
    pub slug: Option<String>,
    pub title: String,
    pub icon_url: Option<String>,
    pub project_type: Option<String>,
    pub viewed_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppState {
    pub onboarding_done: bool,
    pub selected_profile: Option<String>,
    pub last_page: Option<String>,
    pub active_account: Option<String>,
    pub benchmark: Option<BenchmarkResult>,
    pub recently_viewed: Vec<RecentProject>,
    /// "manage" | "optimized" | "modded" | "official" – what the user chose during onboarding
    pub onboarding_choice: Option<String>,
}

pub fn load(paths: &AppPaths) -> AppState {
    util::read_json_opt(&paths.state_file()).ok().flatten().unwrap_or_default()
}

pub fn save(paths: &AppPaths, state: &AppState) -> Result<()> {
    util::write_json(&paths.state_file(), state)
}

impl AppState {
    pub fn push_recent(&mut self, mut item: RecentProject) {
        item.viewed_at = util::now_iso();
        self.recently_viewed.retain(|r| !(r.platform == item.platform && r.id == item.id));
        self.recently_viewed.insert(0, item);
        self.recently_viewed.truncate(30);
    }
}
