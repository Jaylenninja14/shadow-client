use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use pc_core::auth::DeviceCode;
use pc_core::context::Ctx;
use pc_core::minecraft::launch::GameManager;
use pc_core::mods::resolver::InstallPlan;
use pc_core::performance::hardware::HardwareInfo;
use pc_core::settings::Settings;
use tokio_util::sync::CancellationToken;

pub type LogHandle = tracing_subscriber::reload::Handle<tracing_subscriber::filter::LevelFilter, tracing_subscriber::Registry>;

/// A plan computed for the user to confirm. Kept server-side so the UI can't alter URLs/hashes.
#[derive(Clone)]
pub struct PendingPlan {
    pub profile_id: String,
    pub plan: InstallPlan,
}

pub struct AppState {
    pub ctx: Ctx,
    pub settings: RwLock<Settings>,
    pub app: Mutex<pc_core::state::AppState>,
    pub games: Arc<GameManager>,
    pub login: Mutex<Option<(DeviceCode, String, CancellationToken)>>,
    pub plans: Mutex<HashMap<String, PendingPlan>>,
    pub hardware: Mutex<Option<HardwareInfo>>,
    pub log_handle: LogHandle,
    pub _log_guard: tracing_appender::non_blocking::WorkerGuard,
    pub started: Instant,
}

impl AppState {
    pub fn new(ctx: Ctx, settings: Settings, log_handle: LogHandle, guard: tracing_appender::non_blocking::WorkerGuard, started: Instant) -> Self {
        let app = pc_core::state::load(&ctx.paths);
        Self {
            ctx,
            settings: RwLock::new(settings),
            app: Mutex::new(app),
            games: Arc::new(GameManager::default()),
            login: Mutex::new(None),
            plans: Mutex::new(HashMap::new()),
            hardware: Mutex::new(None),
            log_handle,
            _log_guard: guard,
            started,
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap().clone()
    }

    pub fn save_app_state(&self) -> pc_core::Result<()> {
        let s = self.app.lock().unwrap().clone();
        pc_core::state::save(&self.ctx.paths, &s)
    }
}
