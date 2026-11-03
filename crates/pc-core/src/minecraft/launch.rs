//! Launching the game and supervising the process.
//!
//! The launch is only reported as successful once the game's own log shows it reached
//! initialisation (or it has stayed alive for 25 s). If the process exits before that, the
//! launch is reported as failed with a diagnosis taken from its output and crash reports.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

use super::install::Installed;
use super::manifest::{self, RuleEnv};
use crate::auth::LaunchAuth;
use crate::context::Ctx;
use crate::events::emit;
use crate::java::JavaInstall;
use crate::profiles::{LaunchRecord, Profiles};
use crate::{jvmargs, redact, util, Error, Result};

#[derive(Debug, Clone)]
pub struct ArgInputs<'a> {
    pub installed: &'a Installed,
    pub java: &'a JavaInstall,
    pub game_dir: &'a Path,
    pub auth: &'a LaunchAuth,
    pub memory_min_mb: u32,
    pub memory_max_mb: u32,
    pub jvm_preset: &'a str,
    pub extra_jvm_args: Vec<String>,
    pub resolution: Option<(u32, u32)>,
    pub fullscreen: bool,
    pub libraries_dir: &'a Path,
    pub assets_dir: &'a Path,
    /// join this server ("host" or "host:port") right after the game starts
    pub join_server: Option<&'a str>,
}

/// Build the full Java argument vector (everything after the java executable).
pub fn build_args(i: &ArgInputs) -> Vec<String> {
    let v = &i.installed.resolved.json;
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp: Vec<String> = vec![];
    for p in &i.installed.plan.classpath {
        let s = p.to_string_lossy().into_owned();
        if !cp.contains(&s) {
            cp.push(s);
        }
    }
    let classpath = cp.join(sep);
    let game_assets = if let Some(vd) = &i.installed.virtual_assets_dir {
        vd.to_string_lossy().into_owned()
    } else if i.installed.asset_index.as_ref().is_some_and(|a| a.map_to_resources) {
        i.game_dir.join("resources").to_string_lossy().into_owned()
    } else {
        i.assets_dir.to_string_lossy().into_owned()
    };
    let (w, h) = i.resolution.unwrap_or((854, 480));
    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", i.auth.username.clone());
    vars.insert("version_name", i.installed.resolved.id.clone());
    vars.insert("game_directory", i.game_dir.to_string_lossy().into_owned());
    vars.insert("assets_root", i.assets_dir.to_string_lossy().into_owned());
    vars.insert("assets_index_name", i.installed.assets_index.clone().unwrap_or_else(|| "legacy".into()));
    vars.insert("auth_uuid", i.auth.uuid.clone());
    vars.insert("auth_access_token", i.auth.access_token.clone());
    vars.insert("auth_session", format!("token:{}:{}", i.auth.access_token, i.auth.uuid));
    vars.insert("clientid", i.auth.client_id.clone());
    vars.insert("auth_xuid", i.auth.xuid.clone());
    vars.insert("user_type", i.auth.user_type.clone());
    vars.insert("user_properties", "{}".into());
    vars.insert("version_type", v.kind.clone().unwrap_or_else(|| "release".into()));
    vars.insert("resolution_width", w.to_string());
    vars.insert("resolution_height", h.to_string());
    // LWJGL loads e.g. <natives>/windows/x64/org/lwjgl/glfw/glfw.dll with LoadLibrary, which fails
    // past MAX_PATH (error 206), so deep natives folders are passed in their short form.
    vars.insert("natives_directory", shorten(&i.installed.natives_dir, 190).to_string_lossy().into_owned());
    vars.insert("launcher_name", "ShadowClient".into());
    vars.insert("launcher_version", crate::VERSION.into());
    vars.insert("classpath", classpath.clone());
    vars.insert("classpath_separator", sep.into());
    vars.insert("library_directory", i.libraries_dir.to_string_lossy().into_owned());
    vars.insert("game_assets", game_assets);

    let env = RuleEnv::current().with_feature("has_custom_resolution", i.resolution.is_some());

    let mut args: Vec<String> = vec![format!("-Xms{}M", i.memory_min_mb), format!("-Xmx{}M", i.memory_max_mb)];
    args.extend(jvmargs::preset_args(i.jvm_preset, i.java.major));
    args.extend(i.extra_jvm_args.iter().cloned());
    // Log4Shell mitigation for every version (harmless where already patched).
    args.push("-Dlog4j2.formatMsgNoLookups=true".into());
    if let Some((path, argument)) = &i.installed.plan.logging {
        let mut lv = HashMap::new();
        lv.insert("path", path.to_string_lossy().into_owned());
        args.push(manifest::substitute(argument, &lv));
    }
    match v.arguments.as_ref().filter(|a| !a.jvm.is_empty()) {
        Some(a) => args.extend(manifest::expand_args(&a.jvm, &env, &vars)),
        None => {
            if cfg!(windows) {
                args.push("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".into());
            }
            args.push(format!("-Djava.library.path={}", vars["natives_directory"]));
            args.push("-cp".into());
            args.push(classpath);
        }
    }
    args.push(v.main_class.clone().unwrap_or_else(|| "net.minecraft.client.main.Main".into()));
    match (&v.arguments, &v.minecraft_arguments) {
        (Some(a), _) if !a.game.is_empty() => args.extend(manifest::expand_args(&a.game, &env, &vars)),
        (_, Some(legacy)) => {
            args.extend(legacy.split_whitespace().map(|s| manifest::substitute(s, &vars)));
            if let Some((w, h)) = i.resolution {
                args.extend(["--width".into(), w.to_string(), "--height".into(), h.to_string()]);
            }
        }
        _ => {}
    }
    if i.fullscreen {
        args.push("--fullscreen".into());
    }
    if let Some(server) = i.join_server {
        // 1.20+ joins through Quick Play; older versions take --server/--port
        let quick_play = v.arguments.as_ref().is_some_and(|a| serde_json::to_string(&a.game).is_ok_and(|s| s.contains("quickPlayMultiplayer")));
        if quick_play {
            args.extend(["--quickPlayMultiplayer".into(), server.to_string()]);
        } else if let Ok((host, port)) = crate::servers::parse_address(server, 25565) {
            args.extend(["--server".into(), host, "--port".into(), port.to_string()]);
        }
    }
    args
}

/// Text markers showing the client reached initialisation.
const STARTUP_MARKERS: &[&str] = &[
    "Backend library: LWJGL",
    "LWJGL Version",
    "Setting user:",
    "OpenAL initialized",
    "Sound engine started",
    "Reloading ResourceManager",
    "Created: 1024x",
    "Created: 2048x",
];

/// Turn log4j XML events (Mojang's logging config) into plain lines; pass other lines through.
#[derive(Default)]
pub struct LogParser {
    in_event: bool,
    level: String,
    thread: String,
    time: String,
    buf: String,
}

fn attr(line: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = line.find(&key)? + key.len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

impl LogParser {
    pub fn feed(&mut self, line: &str) -> Option<String> {
        if !self.in_event {
            if line.trim_start().starts_with("<log4j:Event") {
                self.in_event = true;
                self.level = attr(line, "level").unwrap_or_default();
                self.thread = attr(line, "thread").unwrap_or_default();
                self.time = attr(line, "timestamp")
                    .and_then(|t| t.parse::<i64>().ok())
                    .and_then(chrono::DateTime::from_timestamp_millis)
                    .map(|d| d.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
                    .unwrap_or_default();
                self.buf.clear();
                if line.contains("</log4j:Event>") {
                    return self.finish(line);
                }
                return None;
            }
            return Some(line.to_string());
        }
        if line.contains("</log4j:Event>") {
            return self.finish(line);
        }
        self.buf.push_str(line);
        self.buf.push('\n');
        None
    }

    fn finish(&mut self, last: &str) -> Option<String> {
        self.buf.push_str(last);
        self.in_event = false;
        let mut msg = String::new();
        let mut rest = self.buf.as_str();
        while let Some(s) = rest.find("<![CDATA[") {
            let after = &rest[s + 9..];
            let e = after.find("]]>").unwrap_or(after.len());
            if !msg.is_empty() {
                msg.push('\n');
            }
            msg.push_str(after[..e].trim_end());
            rest = &after[e.min(after.len())..];
        }
        Some(format!("[{}] [{}/{}]: {}", self.time, self.thread, self.level, msg))
    }
}

/// Explain a failed start from its last output lines.
pub fn diagnose(lines: &[String], exit_code: Option<i32>) -> String {
    let all = lines.join("\n");
    let has = |s: &str| all.contains(s);
    if has("UnsupportedClassVersionError") || has("has been compiled by a more recent version of the Java Runtime") {
        return "The selected Java is too old for this Minecraft version or one of its mods. Choose 'Automatic' Java or install the required runtime.".into();
    }
    if has("Could not reserve enough space") || has("Invalid maximum heap size") || has("Initial heap size set to a larger value") {
        return "Java could not reserve the requested memory. Lower the maximum memory for this profile.".into();
    }
    if has("Incompatible mods found") || has("Mod resolution failed") || has("Incompatible mod set") {
        let detail: Vec<&String> = lines.iter().filter(|l| l.contains("requires") || l.contains("Remove mod") || l.contains("Replace mod") || l.contains("which is missing") || l.contains("incompatible")).take(6).collect();
        let d = detail.iter().map(|s| s.trim().to_string()).collect::<Vec<_>>().join("\n");
        return format!("The mod loader refused to start because of a mod problem:\n{d}\nOpen the Mods page to see warnings, or disable the mods named above.");
    }
    if has("Mixin apply failed") || has("MixinApplyError") || has("InvalidInjectionException") {
        let culprit = lines.iter().find(|l| l.contains("from mod ")).map(|l| l.trim().to_string()).unwrap_or_default();
        return format!("A mod failed to patch the game (Mixin error). This usually means a mod is not compatible with this Minecraft version or with another mod. {culprit}");
    }
    if has("java.lang.OutOfMemoryError") {
        return "Minecraft ran out of memory. Increase the maximum memory a little, or remove heavy mods/resource packs.".into();
    }
    if has("GLFW error") || has("WGL") || has("Pixel format not accelerated") || has("OpenGL") && (has("not supported") || has("Failed to create")) {
        return "The graphics driver could not create an OpenGL context. Update your GPU driver from the manufacturer, and make sure Windows runs Java on the right GPU (Settings → System → Display → Graphics).".into();
    }
    if has("ClassNotFoundException") || has("NoClassDefFoundError") {
        return "A required class was missing. Game files may be incomplete: use Versions → Repair, or check that every mod's dependencies are installed.".into();
    }
    if has("Error: Could not find or load main class") {
        return "Java could not find Minecraft's main class. The installation is incomplete; use Versions → Repair.".into();
    }
    match exit_code {
        Some(c) => format!("Minecraft exited during startup (exit code {c}). See the game log for details."),
        None => "Minecraft exited during startup. See the game log for details.".into(),
    }
}

/// Explain a crash report in plain words: what the game was doing, the mod Fabric suspects, and
/// what usually fixes that kind of crash.
pub fn explain_crash(report: &str) -> String {
    let has = |s: &str| report.contains(s);
    let description = report.lines().find_map(|l| l.strip_prefix("Description: ")).map(str::trim).unwrap_or("");
    // Fabric lists "Suspected Mods:" followed by indented "Name (id), Version: x" lines
    let suspects: Vec<String> = report
        .lines()
        .skip_while(|l| !l.trim_start().starts_with("Suspected Mod"))
        .skip(1)
        .take_while(|l| l.starts_with('\t') || l.starts_with("  "))
        .filter(|l| l.contains('(') && !l.contains("URL"))
        .map(|l| l.trim().split(", Version").next().unwrap_or("").to_string())
        .filter(|s| !s.is_empty())
        .take(3)
        .collect();
    let mixin_mod = report.lines().find_map(|l| l.split("from mod ").nth(1)).map(|s| s.split(|c: char| c == ' ' || c == ']' || c == ')').next().unwrap_or("").to_string());

    let doing = if description.is_empty() { String::new() } else { format!(" while \"{}\"", description.trim_end_matches('.')) };
    let (cause, advice) = if has("java.lang.OutOfMemoryError") {
        ("it ran out of memory".to_string(), "Give the profile a little more memory, or remove heavy mods or resource packs.")
    } else if has("Mixin apply failed") || has("MixinApplyError") || has("InvalidInjectionException") {
        (format!("a mod couldn't patch the game{}", mixin_mod.as_deref().map(|m| format!(" ({m})")).unwrap_or_default()), "Update or remove that mod; it doesn't fit this Minecraft version or clashes with another mod.")
    } else if !suspects.is_empty() {
        (format!("Fabric suspects {}", suspects.join(", ")), "Update, disable or remove that mod on the Mods page, then try again.")
    } else if has("Ticking entity") || has("Ticking block entity") {
        ("an entity or block in the world kept crashing it".to_string(), "Restore a world backup, or remove the mod that adds that entity or block.")
    } else if has("Exception generating new chunk") || has("Feature placement") {
        ("world generation failed".to_string(), "A world-generation mod is usually the cause; update or remove it.")
    } else if has("Chunk file at") || has("Loading NBT data") || has("Couldn't load chunk") {
        ("part of the world couldn't be read".to_string(), "The world may be damaged; restore it from a backup.")
    } else if has("Rendering overlay") || has("Rendering screen") || has("Rendering Item") {
        ("something failed while drawing the screen".to_string(), "Often a resource pack, shader or rendering mod; try without them.")
    } else if has("UnsupportedClassVersionError") {
        ("Java is too old for a mod".to_string(), "Set Java to Automatic for this profile.")
    } else {
        return format!("Minecraft crashed{doing}. Open the crash report for details, or disable recently added mods.");
    };
    format!("Minecraft crashed{doing}: {cause}. {advice}")
}

/// A JVM crash log (hs_err_pid*.log) written since `since`: these are native crashes, usually
/// the graphics driver.
fn native_crash(game_dir: &Path, since: std::time::SystemTime) -> Option<String> {
    let log = std::fs::read_dir(game_dir)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("hs_err_pid"))
        .filter(|e| e.metadata().and_then(|m| m.modified()).is_ok_and(|m| m >= since))
        .map(|e| e.path())
        .next()?;
    let text = std::fs::read_to_string(&log).ok()?;
    let head: String = text.chars().take(200_000).collect();
    let driver = [("nvoglv", "NVIDIA"), ("atio6axx", "AMD"), ("amdxx", "AMD"), ("ig9icd", "Intel"), ("ig7icd", "Intel"), ("ig75icd", "Intel"), ("igxelpicd", "Intel")]
        .iter()
        .find(|(dll, _)| head.contains(dll))
        .map(|(_, vendor)| *vendor);
    Some(match driver {
        Some(v) => format!("Minecraft crashed inside the {v} graphics driver. Update the driver from {v}'s website; if it keeps happening, try turning off shaders or Sodium. Log: {}", log.display()),
        None => format!("Java itself crashed (a native crash). Update your graphics driver and Java; the details are in {}", log.display()),
    })
}

fn explain_crash_file(path: &Path) -> String {
    let text = std::fs::read(path).map(|b| String::from_utf8_lossy(&b[..b.len().min(1 << 20)]).into_owned()).unwrap_or_default();
    format!("{} Crash report: {}", explain_crash(&text), path.display())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub profile_id: String,
    /// "starting" | "running" | "exited" | "crashed" | "failed"
    pub state: String,
    pub pid: Option<u32>,
    pub message: Option<String>,
    pub exit_code: Option<i32>,
    pub crash_report: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub started_at: Option<String>,
}

struct Running {
    pid: u32,
    kill: CancellationToken,
    tail: Arc<Mutex<VecDeque<String>>>,
    log_file: PathBuf,
    started_at: String,
}

#[derive(Default)]
pub struct GameManager {
    running: Mutex<HashMap<String, Running>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningInfo {
    pub profile_id: String,
    pub pid: u32,
    pub started_at: String,
    pub log_file: PathBuf,
}

fn newest_crash_report(game_dir: &Path, since: std::time::SystemTime) -> Option<PathBuf> {
    std::fs::read_dir(game_dir.join("crash-reports"))
        .ok()?
        .flatten()
        .filter_map(|e| {
            let m = e.metadata().ok()?.modified().ok()?;
            (m >= since).then_some((m, e.path()))
        })
        .max_by_key(|(m, _)| *m)
        .map(|(_, p)| p)
}

impl GameManager {
    pub fn is_running(&self, profile_id: &str) -> bool {
        self.running.lock().unwrap().contains_key(profile_id)
    }

    pub fn list(&self) -> Vec<RunningInfo> {
        self.running
            .lock()
            .unwrap()
            .iter()
            .map(|(id, r)| RunningInfo { profile_id: id.clone(), pid: r.pid, started_at: r.started_at.clone(), log_file: r.log_file.clone() })
            .collect()
    }

    pub fn tail(&self, profile_id: &str) -> Vec<String> {
        self.running.lock().unwrap().get(profile_id).map(|r| r.tail.lock().unwrap().iter().cloned().collect()).unwrap_or_default()
    }

    pub fn kill(&self, profile_id: &str) -> bool {
        match self.running.lock().unwrap().get(profile_id) {
            Some(r) => {
                r.kill.cancel();
                true
            }
            None => false,
        }
    }

    /// Spawn Java and wait until the game is confirmed running (or fails).
    pub async fn spawn(
        self: &Arc<Self>,
        ctx: &Ctx,
        profile_id: &str,
        java: &JavaInstall,
        args: Vec<String>,
        game_dir: &Path,
        startup_timeout: Duration,
    ) -> Result<GameState> {
        if self.is_running(profile_id) {
            return Err(Error::Invalid("This profile is already running.".into()));
        }
        std::fs::create_dir_all(game_dir)?;
        let logs = ctx.paths.logs_dir().join("game");
        std::fs::create_dir_all(&logs)?;
        let log_file = util::unique_path(&logs.join(format!("{}-{}.log", util::sanitize_file_name(profile_id), util::timestamp_slug())));
        let mut log = tokio::fs::File::create(&log_file).await?;
        let cmdline = format!("{} {}", java.path.display(), args.join(" "));
        log.write_all(format!("# Shadow Client {} launch\n# {}\n", crate::VERSION, redact::redact(&cmdline)).as_bytes()).await?;
        tracing::info!("launching {profile_id}: {}", redact::redact(&cmdline));

        let mut cmd = tokio::process::Command::new(&java.path);
        cmd.args(&args).current_dir(process_dir(game_dir)).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        let started = std::time::SystemTime::now();
        let mut child = cmd.spawn().map_err(|e| {
            let hint = if cmdline.len() > 32_000 {
                " The launch command is longer than Windows allows; move the data folder to a shorter path."
            } else if process_dir(game_dir).as_os_str().len() >= 260 {
                " The game folder path is longer than Windows allows for a working folder; move it to a shorter path."
            } else {
                ""
            };
            Error::msg(format!("Could not start Java ({}): {e}.{hint}", java.path.display()))
        })?;
        let pid = child.id().unwrap_or(0);
        let started_at = util::now_iso();
        emit(&ctx.sink, "game:state", &GameState { profile_id: profile_id.into(), state: "starting".into(), pid: Some(pid), message: None, exit_code: None, crash_report: None, log_file: Some(log_file.clone()), started_at: Some(started_at.clone()) });

        let tail: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::with_capacity(500)));
        let (line_tx, mut line_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>)].into_iter().flatten() {
            let tx = line_tx.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stream).lines();
                let mut parser = LogParser::default();
                while let Ok(Some(l)) = lines.next_line().await {
                    if let Some(out) = parser.feed(&l) {
                        if tx.send(out).is_err() {
                            break;
                        }
                    }
                }
            });
        }
        drop(line_tx);

        // Writer: log file + ring buffer + startup detection.
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();
        {
            let tail = tail.clone();
            tokio::spawn(async move {
                let mut ready_tx = Some(ready_tx);
                while let Some(line) = line_rx.recv().await {
                    let clean = redact::redact(&line);
                    let _ = log.write_all(clean.as_bytes()).await;
                    let _ = log.write_all(b"\n").await;
                    if ready_tx.is_some() && STARTUP_MARKERS.iter().any(|m| clean.contains(m)) {
                        let _ = ready_tx.take().unwrap().send(());
                    }
                    let mut t = tail.lock().unwrap();
                    if t.len() >= 500 {
                        t.pop_front();
                    }
                    t.push_back(clean);
                }
                let _ = log.flush().await;
            });
        }

        let kill = CancellationToken::new();
        self.running.lock().unwrap().insert(
            profile_id.to_string(),
            Running { pid, kill: kill.clone(), tail: tail.clone(), log_file: log_file.clone(), started_at: started_at.clone() },
        );

        // Wait for: startup marker, early exit, or timeout.
        let t0 = Instant::now();
        let early = tokio::select! {
            r = ready_rx => match r {
                Ok(()) => None,
                // Output closed without a startup marker: the process is exiting (or closed
                // its streams). Never treat that as a successful start.
                Err(_) => tokio::select! {
                    status = child.wait() => Some(status),
                    _ = tokio::time::sleep(startup_timeout.saturating_sub(t0.elapsed())) => None,
                },
            },
            status = child.wait() => Some(status),
            _ = tokio::time::sleep(startup_timeout) => None,
        };
        if let Some(status) = early {
            self.running.lock().unwrap().remove(profile_id);
            tokio::time::sleep(Duration::from_millis(300)).await; // let readers flush
            let code = status.ok().and_then(|s| s.code());
            let lines: Vec<String> = tail.lock().unwrap().iter().cloned().collect();
            let crash = newest_crash_report(game_dir, started);
            let message = diagnose(&lines, code);
            record(ctx, profile_id, "failed", Some(message.clone()), Some(t0.elapsed().as_secs()));
            let st = GameState { profile_id: profile_id.into(), state: "failed".into(), pid: Some(pid), message: Some(message.clone()), exit_code: code, crash_report: crash, log_file: Some(log_file), started_at: Some(started_at) };
            emit(&ctx.sink, "game:state", &st);
            return Err(Error::msg(message));
        }

        let confirmed = t0.elapsed() < startup_timeout;
        let state = GameState {
            profile_id: profile_id.into(),
            state: "running".into(),
            pid: Some(pid),
            message: (!confirmed).then(|| "The game is still starting (no startup message yet).".to_string()),
            exit_code: None,
            crash_report: None,
            log_file: Some(log_file.clone()),
            started_at: Some(started_at.clone()),
        };
        record(ctx, profile_id, "running", None, None);
        emit(&ctx.sink, "game:state", &state);

        // Supervisor: process stats every 2 s, exit handling.
        let me = self.clone();
        let ctx2 = ctx.clone();
        let pid_s = profile_id.to_string();
        let game_dir = game_dir.to_path_buf();
        tokio::spawn(async move {
            let mut sampler = crate::performance::ProcessSampler::new(pid);
            let mut tick = tokio::time::interval(Duration::from_secs(2));
            let status = loop {
                tokio::select! {
                    s = child.wait() => break s.ok(),
                    _ = kill.cancelled() => {
                        let _ = child.kill().await;
                        break child.wait().await.ok();
                    }
                    _ = tick.tick() => {
                        if let Some(sample) = sampler.sample() {
                            emit(&ctx2.sink, "game:stats", &serde_json::json!({"profileId": pid_s, "sample": sample}));
                        }
                    }
                }
            };
            me.running.lock().unwrap().remove(&pid_s);
            let code = status.and_then(|s| s.code());
            let secs = t0.elapsed().as_secs();
            let crash = newest_crash_report(&game_dir, started);
            let (st, msg) = match (code, &crash) {
                (Some(0), None) => ("exited", None),
                (_, Some(p)) => ("crashed", Some(explain_crash_file(p))),
                (c, None) => {
                    let lines: Vec<String> = tail.lock().unwrap().iter().cloned().collect();
                    if kill.is_cancelled() {
                        ("exited", Some("Stopped from the launcher.".to_string()))
                    } else {
                        ("crashed", Some(native_crash(&game_dir, started).unwrap_or_else(|| diagnose(&lines, c))))
                    }
                }
            };
            record(&ctx2, &pid_s, if st == "exited" { "ok" } else { "crashed" }, msg.clone(), Some(secs));
            emit(&ctx2.sink, "game:state", &GameState { profile_id: pid_s, state: st.into(), pid: Some(pid), message: msg, exit_code: code, crash_report: crash, log_file: Some(log_file), started_at: Some(started_at) });
        });
        Ok(state)
    }
}

fn record(ctx: &Ctx, profile_id: &str, result: &str, message: Option<String>, secs: Option<u64>) {
    let profiles = Profiles::new(&ctx.paths);
    if let Ok(mut p) = profiles.get(profile_id) {
        if result == "running" {
            p.last_played = Some(util::now_iso());
        }
        if let Some(s) = secs {
            p.total_play_secs += s;
        }
        p.last_launch = Some(LaunchRecord { at: util::now_iso(), result: result.into(), message, duration_secs: secs });
        let _ = profiles.save(&p);
    }
}

/// Windows can't start a process whose working folder is MAX_PATH (260) or longer, even with
/// long-path support enabled. Deep game folders are given as their 8.3 short form instead.
fn process_dir(dir: &Path) -> PathBuf {
    shorten(dir, 240)
}

/// The 8.3 short form of `dir` when its path is at least `limit` characters (Windows only;
/// unchanged when the volume has no short names).
fn shorten(dir: &Path, limit: usize) -> PathBuf {
    #[cfg(windows)]
    if dir.as_os_str().len() >= limit {
        if let Some(short) = short_path(dir) {
            return short;
        }
    }
    dir.to_path_buf()
}

#[cfg(windows)]
fn short_path(dir: &Path) -> Option<PathBuf> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::Storage::FileSystem::GetShortPathNameW;
    // Paths over MAX_PATH need the \\?\ form for GetShortPathNameW to accept them.
    const VERBATIM: &str = r"\\?\";
    let normalized = dir.to_string_lossy().replace('/', "\\");
    let prefixed = if normalized.starts_with(VERBATIM) { normalized } else { format!("{VERBATIM}{normalized}") };
    let wide: Vec<u16> = std::ffi::OsStr::new(&prefixed).encode_wide().chain(Some(0)).collect();
    // SAFETY: wide is NUL-terminated; the buffer is sized from the first call.
    let len = unsafe { GetShortPathNameW(wide.as_ptr(), std::ptr::null_mut(), 0) };
    if len == 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize];
    let n = unsafe { GetShortPathNameW(wide.as_ptr(), buf.as_mut_ptr(), len) };
    if n == 0 || n >= len {
        return None;
    }
    buf.truncate(n as usize);
    let short = std::ffi::OsString::from_wide(&buf).to_string_lossy().trim_start_matches(VERBATIM).to_string();
    let short = PathBuf::from(short);
    (short.as_os_str().len() < 248).then_some(short)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_reports_are_explained() {
        let fabric = "---- Minecraft Crash Report ----\nDescription: Rendering overlay\n\njava.lang.NullPointerException\n\nSuspected Mods: \n\tFancy Thing (fancything), Version: 1.0\n\t\tIssue tracker URL: https://example.com\n\tat foo\n-- Head --\n";
        let m = explain_crash(fabric);
        assert!(m.contains("while \"Rendering overlay\"") && m.contains("Fabric suspects Fancy Thing (fancything)"), "{m}");
        assert!(!m.contains("Issue tracker"));
        assert!(explain_crash("Description: Ticking entity\njava.lang.OutOfMemoryError: Java heap space").contains("out of memory"));
        let mixin = explain_crash("Description: Initializing game\nMixin apply failed sodium.mixins.json:Foo from mod sodium -> net.minecraft.Bar");
        assert!(mixin.contains("couldn't patch the game (sodium)"), "{mixin}");
        assert!(explain_crash("Description: Ticking entity\nsomething").contains("entity or block"));
        assert!(explain_crash("nothing useful").starts_with("Minecraft crashed. Open the crash report"));
    }

    #[test]
    fn native_driver_crash_is_named() {
        let d = tempfile::tempdir().unwrap();
        let since = std::time::SystemTime::now() - Duration::from_secs(5);
        assert!(native_crash(d.path(), since).is_none());
        std::fs::write(d.path().join("hs_err_pid1234.log"), "# A fatal error has been detected\n# C  [nvoglv64.dll+0x123]").unwrap();
        let m = native_crash(d.path(), since).unwrap();
        assert!(m.contains("NVIDIA graphics driver"), "{m}");
    }

    #[cfg(windows)]
    #[test]
    fn deep_game_folders_get_a_short_working_dir() {
        let d = tempfile::tempdir().unwrap();
        let mut deep = d.path().to_path_buf();
        while deep.as_os_str().len() < 280 {
            deep.push("a-rather-long-folder-name-for-testing");
        }
        std::fs::create_dir_all(&deep).unwrap();
        let dir = process_dir(&deep);
        if dir == deep {
            eprintln!("8.3 names unavailable on this volume; nothing to shorten");
            return;
        }
        eprintln!("short working dir: {}", dir.display());
        assert!(dir.as_os_str().len() < 248);
        std::fs::write(deep.join("marker"), b"x").unwrap();
        assert!(dir.join("marker").is_file(), "short path points at the same folder");
        let short = d.path().join("short");
        assert_eq!(process_dir(&short), short, "normal paths are left alone");
    }

    #[test]
    fn parses_log4j_xml() {
        let mut p = LogParser::default();
        assert!(p.feed(r#"<log4j:Event logger="ehf" timestamp="1700000000000" level="INFO" thread="Render thread">"#).is_none());
        assert!(p.feed("  <log4j:Message><![CDATA[Setting user: Steve]]></log4j:Message>").is_none());
        let out = p.feed("</log4j:Event>").unwrap();
        assert!(out.contains("[Render thread/INFO]: Setting user: Steve"), "{out}");
        assert_eq!(p.feed("plain line").as_deref(), Some("plain line"));
    }

    #[test]
    fn diagnoses_common_failures() {
        let l = |s: &str| vec![s.to_string()];
        assert!(diagnose(&l("java.lang.UnsupportedClassVersionError: x"), Some(1)).contains("Java is too old"));
        assert!(diagnose(&l("Error: Could not reserve enough space for 8388608KB object heap"), Some(1)).contains("memory"));
        assert!(diagnose(&vec!["Incompatible mods found!".into(), " - Mod 'X' (x) 1.0 requires any version of fabric-api, which is missing!".into()], Some(1)).contains("which is missing"));
        assert!(diagnose(&l("something else"), Some(-1)).contains("exit code -1"));
    }
}
