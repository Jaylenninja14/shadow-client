// Typed wrappers around the Rust commands. Every call goes to real engine code.
import { invoke } from "@tauri-apps/api/core";

export type LoaderKind = "vanilla" | "fabric" | "quilt" | "forge" | "neoforge";
export type ContentKind = "mod" | "resourcepack" | "shader";
export type ProfileKind = "vanilla" | "performance" | "pvp" | "survival" | "modded" | "custom" | "external";

export class ApiError extends Error {
  kind: string;
  constructor(kind: string, message: string) {
    super(message);
    this.kind = kind;
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    if (e && typeof e === "object" && "message" in e) {
      const err = e as { kind?: string; message: string };
      throw new ApiError(err.kind ?? "error", err.message);
    }
    throw new ApiError("error", String(e));
  }
}

export interface GraphicsSettings {
  renderDistance: number; simulationDistance: number; particles: number; maxFps: number; vsync: boolean;
  graphics: "fast" | "fancy"; clouds: "off" | "fast" | "fancy"; entityShadows: boolean; mipmapLevels: number;
  biomeBlend: number; entityDistance: number; smoothLighting: boolean;
}

export interface Settings {
  theme: "dark" | "light"; accent: string; language: string; startupPage: "home" | "last";
  checkModUpdatesOnStartup: boolean; notifications: boolean; downloadDir: string | null; minecraftDir: string | null; officialOnlyThis: boolean;
  extraScanDirs: string[]; defaultVersion: string | null; defaultLoader: LoaderKind; defaultProfile: string | null;
  javaPath: string | null; memoryMinMb: number | null; memoryMaxMb: number | null; jvmArgs: string;
  jvmPreset: "mojang" | "minimal" | "zgc"; resolutionWidth: number | null; resolutionHeight: number | null;
  fullscreen: boolean; onGameLaunch: "keep" | "minimize" | "hide"; showSnapshots: boolean; showOldVersions: boolean;
  defaultPreset: "auto" | "low" | "balanced" | "high" | "custom"; customGraphics: GraphicsSettings;
  autoInstallSodium: boolean; performanceMods: string[];
  downloadConcurrency: number; downloadRetries: number; cacheTtlMinutes: number;
  debugLogging: boolean; experimentalForgeLaunch: boolean; promptBenchmarkOnFirstRun: boolean;
  msaClientId: string | null; modrinthContact: string | null;
}

export interface RecentProject { platform: string; id: string; slug: string | null; title: string; iconUrl: string | null; projectType: string | null; viewedAt: string }
export interface AppStateT {
  onboardingDone: boolean; selectedProfile: string | null; lastPage: string | null; activeAccount: string | null;
  benchmark: BenchmarkResult | null; recentlyViewed: RecentProject[]; onboardingChoice: string | null;
}
export interface AppInfo { version: string; dataDir: string; portable: boolean; logsDir: string; minecraftDir: string; startupMs: number }

export interface LaunchRecord { at: string; result: "running" | "ok" | "crashed" | "failed"; message: string | null; durationSecs: number | null }
export interface Profile {
  id: string; name: string; kind: ProfileKind; mcVersion: string; loader: LoaderKind; loaderVersion: string | null;
  externalVersionId: string | null; gameDir: string | null; externalRoot: string | null; external: boolean;
  javaPath: string | null; memoryMinMb: number | null; memoryMaxMb: number | null; jvmArgs: string | null;
  resolutionWidth: number | null; resolutionHeight: number | null; fullscreen: boolean | null; preset: string | null;
  builtinMods: boolean; icon: string | null; createdAt: string; lastPlayed: string | null; lastLaunch: LaunchRecord | null;
  totalPlaySecs: number; notes: string | null;
}
export interface BedrockView { installed: boolean; version: string | null; dataDir: string | null; curseforgeKey: boolean; storeUrl: string }
export interface BedrockPack { kind: "resource" | "behavior" | "skin"; name: string; description: string; uuid: string; version: string; folder: string; development: boolean }
export interface SavedServer { id: string; name: string; address: string }
export interface ServerStatus { online: boolean; pingMs: number | null; motd: string; version: string; playersOnline: number | null; playersMax: number | null; icon: string | null; error: string | null }
export interface UpdateInfo { current: string; latest: string | null; newer: boolean; url: string | null; notes: string | null; installer: { url: string; sha256: string; size: number } | null; msi?: { url: string; sha256: string; size: number } | null; canInstall: boolean }
/** A share link's pack: name, Minecraft version, loader and its Modrinth items (kind m/r/s, project, version, title). */
export interface SharePack { f: number; n: string; m: string; l: string; i: { k: "m" | "r" | "s"; p: string; v: string; t: string; s?: "c" }[] }
export interface ShareExport { code: string; link: string; included: string[]; skipped: string[] }
export interface Screenshot { name: string; path: string; size: number; taken: string | null }
export interface BedrockWorld { folder: string; name: string; size: number; played: string | null; icon: string | null }
export interface WorldBackup { file: string; path: string; size: number; created: string | null }
export interface BedrockServer { index: number; name: string; address: string; port: number }
export interface BedrockModule {
  id: string;
  name: string;
  tab: "performance" | "pvp" | "visual" | "hud" | "audio" | "game" | "privacy";
  icon: string;
  about: string;
  kind: "toggle" | "cycle";
  source: "settings" | "pack";
  enabled: boolean;
  value: string;
  valueLabel: string;
  choices: { value: string; label: string }[];
  available: boolean;
}
export interface BedrockMenu {
  users: { id: string; label: string; lastPlayed: string | null }[];
  user: string | null;
  running: boolean;
  modules: BedrockModule[];
  packActive: boolean;
  hasOriginal: boolean;
}
export interface BedrockImportReport { installed: string[]; replaced: string[]; skipped: string[]; openedInBedrock: boolean }
export interface Hotkey { id: string; label: string; group: string; defaultKey: string; key: string }
export interface HudModule { key: string; name: string; description: string; enabled: boolean }
export interface HudState { configured: boolean; modules: HudModule[]; rightClickCps: boolean }
export interface NewProfile { name: string; kind: ProfileKind; mcVersion: string; loader: LoaderKind; loaderVersion: string | null; gameDir: string | null; preset: string | null; builtinMods: boolean | null; ingameFeatures?: boolean | null }
export interface SetupReport { installed: string[]; notes: string[]; sodium: string | null; suggestionVersions: string[] }
export interface DeleteReport { profileRemoved: boolean; preserved: string[]; recycled: string | null; parked: string | null; message: string }

export interface Account { id: string; username: string; kind: "msa" | "offline"; xuid: string | null; clientId: string | null; addedAt: string }
export interface DeviceCode { userCode: string; verificationUri: string; expiresIn: number; interval: number; message: string }

export interface Problem { severity: "error" | "warning" | "info"; title: string; detail: string; action: string | null }
export interface JavaInstall { path: string; home: string; version: string; major: number; vendor: string | null; arch: string; source: string }
export interface Preflight {
  ready: boolean; problems: Problem[]; versionId: string | null; requiredJava: number | null; java: JavaInstall | null;
  memoryMinMb: number; memoryMaxMb: number; memoryReasons: string[]; modCount: number; enabledModCount: number;
  account: Account | null; filesMissing: number; gameDir: string;
}
export interface GameState { profileId: string; state: "starting" | "running" | "exited" | "crashed" | "failed"; pid: number | null; message: string | null; exitCode: number | null; crashReport: string | null; logFile: string | null; startedAt: string | null }
export interface ProcessSample { pid: number; memoryMb: number; cpuPercent: number; at: string }
export interface RunningInfo { profileId: string; pid: number; startedAt: string; logFile: string }

export interface JobInfo {
  id: string; title: string; status: "queued" | "running" | "completed" | "failed" | "cancelled"; totalFiles: number; doneFiles: number;
  skippedFiles: number; totalBytes: number | null; downloadedBytes: number; bytesPerSec: number; error: string | null; failed: string[];
  startedAt: string; finishedAt: string | null; retryable: boolean;
}

export interface ModMeta { modId: string; name: string; version: string; description: string | null; authors: string[]; loader: string; depends: Record<string, string[]>; breaks: Record<string, string[]>; iconPath: string | null; environment: string | null; license: string | null; homepage: string | null; provides: string[] }
export interface IndexEntry { fileName: string; kind: ContentKind; sha1: string; sha512: string | null; source: string; projectId: string | null; versionId: string | null; slug: string | null; title: string | null; versionNumber: string | null; iconUrl: string | null; websiteUrl: string | null; explicit: boolean; builtin: boolean; installedAt: string; requiredBy: string[]; gameVersions: string[]; loaders: string[] }
export interface IdentifiedProject { projectId: string; versionId: string; title: string | null; slug: string | null; iconUrl: string | null; versionNumber: string }
export interface InstalledMod { fileName: string; path: string; enabled: boolean; size: number; sha1: string; meta: ModMeta | null; icon: string | null; source: IndexEntry | null; identified: IdentifiedProject | null; update: { versionId: string; versionNumber: string; platform: string } | null; warnings: string[]; builtin: boolean }
export interface BackupEntry { fileName: string; path: string; takenAt: string; title: string | null }

export interface PlannedFile { platform: string; projectId: string; versionId: string; title: string; slug: string | null; versionNumber: string; fileName: string; url: string | null; sha1: string | null; sha512: string | null; size: number | null; reason: string; requiredBy: string | null; replaces: string | null; iconUrl: string | null; websiteUrl: string | null; downloadable: boolean; embedded: string | null; builtin: boolean; gameVersions: string[]; loaders: string[]; kind: ContentKind }
export interface Conflict { a: string; b: string; reason: string; severity: string }
export interface InstallPlan { target: { mcVersion: string; loader: LoaderKind } | null; files: PlannedFile[]; optional: { platform: string; projectId: string; title: string; requiredBy: string }[]; alreadyInstalled: string[]; conflicts: Conflict[]; warnings: string[]; blocked: boolean; blockedReason: string | null; suggestion: { mcVersions: string[]; loaders: string[] } | null; totalSize: number | null; manualDownloads: PlannedFile[] }
export interface PlanResult { planId: string; plan: InstallPlan }
export interface InstallReport { installed: string[]; skipped: string[]; backups: string[]; warnings: string[]; manual: PlannedFile[]; restartRequired: boolean }

// Modrinth (snake_case as returned by the API)
export interface MrHit { project_id: string; slug: string | null; title: string; description: string; author: string | null; icon_url: string | null; downloads: number | null; follows: number | null; categories: string[]; display_categories: string[]; versions: string[]; latest_version: string | null; date_modified: string | null; project_type: string | null; client_side: string | null; server_side: string | null; license: string | null }
export interface MrSearchResult { hits: MrHit[]; offset: number; limit: number; total_hits: number }
export interface MrSearchQuery { query: string; projectType: string; gameVersion: string | null; loader: string | null; categories: string[]; sort: string | null; offset: number; limit: number; clientSideOnly: boolean }
export interface MrProject { id: string; slug: string | null; title: string; description: string; body: string | null; categories: string[]; client_side: string | null; server_side: string | null; project_type: string; downloads: number | null; followers: number | null; icon_url: string | null; license: { id: string | null; name: string | null; url: string | null } | null; game_versions: string[]; loaders: string[]; source_url: string | null; issues_url: string | null; wiki_url: string | null; updated: string | null; published: string | null }
export interface MrVersion { id: string; project_id: string; name: string | null; version_number: string; version_type: string; game_versions: string[]; loaders: string[]; date_published: string; downloads: number | null; dependencies: { version_id: string | null; project_id: string | null; dependency_type: string }[]; files: { hashes: { sha1: string | null; sha512: string | null }; url: string; filename: string; primary: boolean; size: number | null }[] }
export interface ProjectDetails { project: MrProject; authors: string[]; url: string }

// CurseForge
export interface CfMod { id: number; name: string; slug: string; summary: string; downloadCount: number | null; links: { websiteUrl: string | null; sourceUrl: string | null; issuesUrl: string | null } | null; logo: { thumbnailUrl: string | null; url: string | null } | null; authors: { name: string }[]; categories: { id: number; name: string; slug: string | null }[]; dateModified: string | null; latestFilesIndexes: { gameVersion: string; fileId: number; filename: string; releaseType: number; modLoader: number | null }[]; allowModDistribution: boolean | null; classId: number | null }
export interface CfSearchResult { data: CfMod[]; pagination: { index: number; pageSize: number; resultCount: number; totalCount: number } }
export interface CfSearchQuery { query: string; classId: number; gameVersion: string | null; loader: string | null; categoryId: number | null; sort: string | null; offset: number; limit: number }

// Detection
export interface LauncherProfile { key: string; name: string; kind: string; lastVersionId: string | null; gameDir: string | null; resolvedGameDir: string; javaDir: string | null; javaArgs: string | null; lastUsed: string | null }
export interface DetectedVersion { id: string; inheritsFrom: string | null; loader: string; loaderVersion: string | null; hasJar: boolean; kind: string | null }
export interface GameDirContents { path: string; exists: boolean; mods: string[]; resourcepacks: string[]; shaderpacks: string[]; worlds: { folder: string; lastModified: string | null }[]; screenshots: number; hasConfig: boolean; hasOptions: boolean }
export interface DetectedInstall { root: string; source: string; label: string; launcherProfiles: LauncherProfile[]; versions: DetectedVersion[]; gameDirs: GameDirContents[]; instanceVersion: string | null; instanceLoader: string | null; notes: string[] }
export interface OfficialLauncher { installed: boolean; kind: "store" | "classic" | "none"; path: string | null; minecraftDir: string; minecraftDirExists: boolean }

export interface VersionSummary { id: string; kind: string; releaseTime: string; installed: boolean }
export interface VersionList { latestRelease: string; latestSnapshot: string; versions: VersionSummary[] }

// Performance
export interface GpuInfo { name: string; vendor: string; driverVersion: string | null; vramMb: number | null; integrated: boolean }
export interface HardwareInfo { cpuName: string; cpuPhysicalCores: number | null; cpuLogicalCores: number; cpuMhz: number | null; ramTotalMb: number; ramAvailableMb: number; gpus: GpuInfo[]; display: { width: number; height: number; refreshHz: number | null } | null; osName: string | null; osVersion: string | null; freeDiskMb: number | null }
export interface BenchmarkResult { at: string; cpuSingle: number; cpuMulti: number; threads: number; memoryGbps: number; diskWriteMbps: number | null; diskReadMbps: number | null; durationMs: number; hardware: HardwareInfo; recommendedPreset: string; reasons: string[] }
export interface PresetInfo { id: string; name: string; description: string }
export interface PresetPreview { graphics: GraphicsSettings; changes: [string, string, string][]; optionsExists: boolean; warnings: string[] }
export interface ApplyReport { changed: [string, string, string][]; backup: string | null; created: boolean }
export interface OptionsBackup { path: string; name: string; original: boolean }
export interface PerfModStatus { slug: string; projectId: string; name: string; role: string; summary: string; recommended: boolean; availableVersion: string | null; available: boolean | null; installedFile: string | null; enabled: boolean; note: string | null }
export interface BuiltinStatus { key: string; name: string; modId: string; modrinthProjectId: string; modrinthSlug: string; license: string; authors: string[]; category: string; summary: string; note: string | null; pinned: { gameVersion: string; fileName: string }[]; installedFile: string | null; enabled: boolean; pinnedForProfile: boolean }
export interface MemoryRecommendation { minMb: number; maxMb: number; ceilingMb: number; reasons: string[] }

export interface PackInfo { fileName: string; isFolder: boolean; description: string | null; packFormat: number | null; icon: string | null; active: boolean; size: number }
export interface ShaderState { packs: PackInfo[]; irisConfigFound: boolean; enabled: boolean; activePack: string | null }
export interface PackSummary { format: string; name: string; version: string; summary: string | null; mcVersion: string; loader: LoaderKind; loaderVersion: string | null; fileCount: number; overrideCount: number; totalSize: number | null; warnings: string[]; supported: boolean }
export interface PackInstallReport { profileId: string; installedFiles: number; overrides: number; skipped: string[]; manual: { name: string; fileName: string; url: string; targetFolder: string }[]; warnings: string[]; backup: string | null }
export interface FeatureStatus { key: string; name: string; mod: string; category: string; projectId: string; slug: string; inPack: boolean; suite: boolean; coveredBySuite: boolean; legacy?: boolean; summary: string; note: string | null; available: boolean | null; availableVersion: string | null; installedFile: string | null; enabled: boolean; overlapsSuite: boolean; checkNote: string | null }
export interface FeaturesView { features: FeatureStatus[]; suiteProvides: string[]; pack: string[] }
export interface GameOption { key: string; label: string; description: string; kind: "bool" | "percent"; value: string | null }
export interface SkinEntry { id: string; name: string; model: "classic" | "slim"; createdAt: string; updatedAt: string; source: string }
export interface SkinWithData extends SkinEntry { dataUrl: string }
export interface LogFile { name: string; path: string; size: number; modified: string | null; kind: string }

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  appReady: () => call<number>("app_ready"),
  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  resetSettings: () => call<Settings>("reset_settings"),
  getAppState: () => call<AppStateT>("get_app_state"),
  setAppState: (patch: Partial<Pick<AppStateT, "onboardingDone" | "onboardingChoice" | "selectedProfile" | "lastPage">>) => call<AppStateT>("set_app_state", { patch }),

  detectInstalls: () => call<DetectedInstall[]>("detect_installs"),
  officialLauncher: () => call<OfficialLauncher>("official_launcher"),
  openOfficialLauncher: () => call<string>("open_official_launcher"),
  /** Opens the official launcher with this profile and its mods added as an installation. */
  openOfficialWithProfile: (profileId?: string) => call<string>("open_official_with_profile", { profileId: profileId ?? null }),
  importInstall: (req: { root: string; gameDir: string; versionId: string | null; mcVersion: string | null; loader: string | null; name: string; mode: "manage" | "copy"; includeWorlds: boolean }) => call<Profile>("import_install", { req }),
  registerWithOfficial: (profileId: string) => call<{ profileKey: string; versionId: string; backup: string | null; hidden: number }>("register_with_official", { profileId }),

  listVersions: (snapshots: boolean, old: boolean, refresh = false) => call<VersionList>("list_versions", { snapshots, old, refresh }),
  loaderGameVersions: (loader: string) => call<{ version: string; stable: boolean }[]>("loader_game_versions", { loader }),
  loaderVersions: (loader: string, mc: string) => call<{ version: string; stable: boolean }[]>("loader_versions", { loader, mc }),
  recommendedPerformanceVersion: () => call<string>("recommended_performance_version"),
  repairProfile: (profileId: string) => call<string>("repair_profile", { profileId }),

  listProfiles: () => call<Profile[]>("list_profiles"),
  getProfile: (id: string) => call<Profile>("get_profile", { id }),
  createProfile: (req: NewProfile) => call<{ profile: Profile; report: SetupReport }>("create_profile", { req }),
  updateProfile: (profile: Profile) => call<Profile>("update_profile", { profile }),
  renameProfile: (id: string, name: string) => call<Profile>("rename_profile", { id, name }),
  duplicateProfile: (id: string, name: string, includeWorlds: boolean) => call<Profile>("duplicate_profile", { id, name, includeWorlds }),
  deleteProfile: (id: string) => call<DeleteReport>("delete_profile", { id }),
  exportProfile: (id: string, dest: string, includeOptions: boolean) => call<string>("export_profile", { id, dest, includeOptions }),
  checkVersionChange: (id: string, mc: string) => call<{ ok: boolean; unavailable: string[]; unknown: string[]; message: string }>("check_version_change", { id, mc }),
  applyVersionChange: (id: string, mc: string) => call<Profile>("apply_version_change", { id, mc }),
  openProfileFolder: (id: string, sub?: string) => call<void>("open_profile_folder", { id, sub: sub ?? null }),

  listAccounts: () => call<Account[]>("list_accounts"),
  addOfflineAccount: (name: string) => call<Account>("add_offline_account", { name }),
  removeAccount: (id: string) => call<void>("remove_account", { id }),
  setActiveAccount: (id: string) => call<void>("set_active_account", { id }),
  msaStart: () => call<DeviceCode>("msa_start"),
  msaCancel: () => call<void>("msa_cancel"),
  curseforgeStatus: () => call<boolean>("curseforge_status"),
  setCurseforgeKey: (key: string) => call<void>("set_curseforge_key", { key }),
  clearCurseforgeKey: () => call<void>("clear_curseforge_key"),

  preflight: (profileId: string) => call<Preflight>("preflight", { profileId }),
  play: (profileId: string, force: boolean) => call<GameState>("play", { profileId, force }),
  runningGames: () => call<RunningInfo[]>("running_games"),
  killGame: (profileId: string) => call<boolean>("kill_game", { profileId }),
  gameLogTail: (profileId: string) => call<string[]>("game_log_tail", { profileId }),
  listJava: () => call<JavaInstall[]>("list_java"),
  downloadJava: (component: string) => call<JavaInstall>("download_java", { component }),
  memoryRecommendation: (profileId: string | null) => call<MemoryRecommendation>("memory_recommendation", { profileId }),

  modsList: (profileId: string, identify: boolean) => call<InstalledMod[]>("mods_list", { profileId, identify }),
  modsCheckUpdates: (profileId: string) => call<InstalledMod[]>("mods_check_updates", { profileId }),
  modrinthSearch: (query: MrSearchQuery) => call<MrSearchResult>("modrinth_search", { query }),
  modrinthProject: (id: string) => call<ProjectDetails>("modrinth_project", { id }),
  modrinthVersions: (id: string, profileId: string | null) => call<MrVersion[]>("modrinth_versions", { id, profileId }),
  curseforgeSearch: (query: CfSearchQuery) => call<CfSearchResult>("curseforge_search", { query }),
  curseforgeMod: (id: number) => call<CfMod>("curseforge_mod", { id }),
  planInstall: (req: { profileId: string; platform: "modrinth" | "curseforge"; projectId: string; versionId: string | null; kind: ContentKind }) => call<PlanResult>("plan_install", { req }),
  planUpdate: (profileId: string, fileName: string) => call<PlanResult>("plan_update", { profileId, fileName }),
  executePlan: (planId: string, optional: string[]) => call<InstallReport>("execute_plan", { req: { planId, optional } }),
  discardPlan: (planId: string) => call<void>("discard_plan", { planId }),
  modSetEnabled: (profileId: string, fileName: string, enabled: boolean, kind: ContentKind = "mod") => call<string>("mod_set_enabled", { profileId, fileName, enabled, kind }),
  modRemove: (profileId: string, fileName: string, kind: ContentKind = "mod") => call<{ backup: string; dependents: string[] }>("mod_remove", { profileId, fileName, kind }),
  contentRemove: (profileId: string, fileName: string, kind: ContentKind) => call<{ backup: string; dependents: string[] }>("content_remove", { profileId, fileName, kind }),
  modBackups: (profileId: string, kind: ContentKind = "mod") => call<BackupEntry[]>("mod_backups", { profileId, kind }),
  modRestore: (profileId: string, path: string, kind: ContentKind = "mod") => call<string>("mod_restore", { profileId, path, kind }),
  recordRecent: (item: RecentProject) => call<void>("record_recent", { item }),
  builtinMods: (profileId: string | null) => call<BuiltinStatus[]>("builtin_mods", { profileId }),
  installBuiltins: (profileId: string) => call<SetupReport>("install_builtins", { profileId }),
  performanceModsStatus: (profileId: string) => call<PerfModStatus[]>("performance_mods_status", { profileId }),
  installPerformanceMods: (profileId: string, slugs: string[]) => call<PlanResult>("install_performance_mods", { profileId, slugs }),

  resourcepacksList: (profileId: string) => call<PackInfo[]>("resourcepacks_list", { profileId }),
  resourcepackSetActive: (profileId: string, name: string, active: boolean) => call<void>("resourcepack_set_active", { profileId, name, active }),
  shaderState: (profileId: string) => call<ShaderState>("shader_state", { profileId }),
  shaderSet: (profileId: string, name: string | null) => call<void>("shader_set", { profileId, name }),

  hardwareInfo: (refresh: boolean) => call<HardwareInfo>("hardware_info", { refresh }),
  runBenchmark: () => call<BenchmarkResult>("run_benchmark"),
  presetList: () => call<PresetInfo[]>("preset_list"),
  presetPreview: (profileId: string, preset: string) => call<PresetPreview>("preset_preview", { profileId, preset }),
  applyPreset: (profileId: string, preset: string) => call<ApplyReport>("apply_preset", { profileId, preset }),
  graphicsWarnings: (graphics: GraphicsSettings) => call<string[]>("graphics_warnings", { graphics }),
  currentGraphics: (profileId: string) => call<Record<string, string> | null>("current_graphics", { profileId }),
  optionsBackups: (profileId: string) => call<OptionsBackup[]>("options_backups", { profileId }),
  optionsRestore: (profileId: string, name: string) => call<void>("options_restore", { profileId, name }),

  modpackInspect: (path: string) => call<PackSummary>("modpack_inspect", { path }),
  modpackInstallFile: (path: string, name: string | null) => call<{ profile: Profile; report: PackInstallReport }>("modpack_install_file", { path, name }),
  modpackInstallRemote: (platform: "modrinth" | "curseforge", projectId: string, versionId: string | null) => call<{ profile: Profile; report: PackInstallReport }>("modpack_install_remote", { req: { platform, projectId, versionId } }),
  modpackUpdateFile: (profileId: string, path: string) => call<PackInstallReport>("modpack_update_file", { profileId, path }),

  downloadsList: () => call<JobInfo[]>("downloads_list"),
  downloadCancel: (id: string) => call<boolean>("download_cancel", { id }),
  downloadRetry: (id: string) => call<JobInfo>("download_retry", { id }),
  downloadsClear: () => call<void>("downloads_clear"),

  logsList: () => call<LogFile[]>("logs_list"),
  logRead: (path: string, forSharing: boolean) => call<string>("log_read", { path, forSharing }),
  diagnosticsReport: () => call<string>("diagnostics_report"),
  cacheInfo: () => call<{ httpCacheBytes: number; tmpBytes: number }>("cache_info"),
  clearCache: () => call<number>("clear_cache"),
  openPath: (path: string, reveal = false) => call<void>("open_path", { path, reveal }),
  openUrl: (url: string) => call<void>("open_url", { url }),
  openDataDir: () => call<void>("open_data_dir"),

  featuresStatus: (profileId: string) => call<FeaturesView>("features_status", { profileId }),
  featuresPlan: (profileId: string, keys: string[]) => call<PlanResult>("features_plan", { profileId, keys }),
  gameOptionsGet: (profileId: string) => call<GameOption[] | null>("game_options_get", { profileId }),
  gameOptionsSet: (profileId: string, key: string, value: string) => call<void>("game_options_set", { profileId, key, value }),

  bedrockInfo: () => call<BedrockView>("bedrock_info"),
  bedrockLaunch: () => call<void>("bedrock_launch"),
  bedrockPacks: () => call<BedrockPack[]>("bedrock_packs"),
  bedrockImport: (path: string) => call<BedrockImportReport>("bedrock_import", { path }),
  bedrockRemove: (kind: string, folder: string, development: boolean) => call<string>("bedrock_remove", { kind, folder, development }),
  bedrockOpenFolder: (kind: string | null) => call<void>("bedrock_open_folder", { kind }),
  bedrockCfClasses: () => call<{ id: number; name: string; slug: string | null }[]>("bedrock_cf_classes"),
  bedrockCfSearch: (classId: number, query: string, sort: string | null, offset: number) => call<CfSearchResult>("bedrock_cf_search", { classId, query, sort, offset }),
  bedrockCfInstall: (modId: number) => call<BedrockImportReport>("bedrock_cf_install", { modId }),
  bedrockMenu: (user: string | null) => call<BedrockMenu>("bedrock_menu", { user }),
  serversList: () => call<SavedServer[]>("servers_list"),
  serversSave: (servers: SavedServer[]) => call<SavedServer[]>("servers_save", { servers }),
  serverPingJava: (address: string) => call<ServerStatus>("server_ping_java", { address }),
  serverPingBedrock: (address: string) => call<ServerStatus>("server_ping_bedrock", { address }),
  playJoin: (profileId: string, address: string) => call<GameState>("play_join", { profileId, address }),
  updateCheck: () => call<UpdateInfo>("update_check"),
  updateInstall: () => call<void>("update_install"),
  sharePending: () => call<string | null>("share_pending"),
  shareExport: (profileId: string) => call<ShareExport>("share_export", { profileId }),
  shareRead: (link: string) => call<SharePack>("share_read", { link }),
  shareImport: (link: string, name: string) => call<{ profile: Profile; plan: PlanResult }>("share_import", { link, name }),
  screenshotsList: (profileId: string) => call<Screenshot[]>("screenshots_list", { profileId }),
  screenshotRemove: (profileId: string, name: string) => call<string>("screenshot_remove", { profileId, name }),
  bedrockWorlds: (user: string | null) => call<BedrockWorld[]>("bedrock_worlds", { user }),
  bedrockWorldBackup: (user: string | null, folder: string) => call<string>("bedrock_world_backup", { user, folder }),
  bedrockWorldBackups: () => call<WorldBackup[]>("bedrock_world_backups"),
  bedrockWorldRestore: (path: string) => call<BedrockImportReport>("bedrock_world_restore", { path }),
  bedrockServers: (user: string | null) => call<BedrockServer[]>("bedrock_servers", { user }),
  bedrockServerAdd: (user: string | null, name: string, address: string, port: number) => call<BedrockServer[]>("bedrock_server_add", { user, name, address, port }),
  bedrockServerRemove: (user: string | null, index: number) => call<BedrockServer[]>("bedrock_server_remove", { user, index }),
  bedrockJoin: (address: string, port: number) => call<void>("bedrock_join", { address, port }),
  bedrockMenuSet: (user: string | null, id: string, value: string) => call<BedrockModule>("bedrock_menu_set", { user, id, value }),
  bedrockMenuRestore: (user: string | null) => call<void>("bedrock_menu_restore", { user }),
  serverPluginInstall: (dir: string) => call<string>("server_plugin_install", { dir }),
  hotkeysGet: (profileId: string) => call<Hotkey[] | null>("hotkeys_get", { profileId }),
  hotkeysSet: (profileId: string, id: string, key: string) => call<void>("hotkeys_set", { profileId, id, key }),
  hudState: (profileId: string) => call<HudState>("hud_state", { profileId }),
  hudApply: (profileId: string, enabled: string[], rightClickCps: boolean | null) => call<void>("hud_apply", { profileId, enabled, rightClickCps }),
  menuReadImage: (path: string) => call<string>("menu_read_image", { path }),
  menuState: (profileId: string) => call<{ installed: boolean; background: string | null }>("menu_state", { profileId }),
  menuApply: (profileId: string, images: { background: string; logo: string; button: string; buttonHover: string }) => call<void>("menu_apply", { profileId, images }),
  menuRemove: (profileId: string) => call<void>("menu_remove", { profileId }),

  skinsList: () => call<SkinWithData[]>("skins_list"),
  skinSave: (req: { id: string | null; name: string; model: string; dataUrl: string; source: string | null }) => call<SkinEntry>("skin_save", { req }),
  skinDelete: (id: string) => call<void>("skin_delete", { id }),
  skinExport: (id: string, dest: string) => call<void>("skin_export", { id, dest }),
  skinReadFile: (path: string) => call<string>("skin_read_file", { path }),
  skinFetchPlayer: (name: string) => call<{ name: string; model: "classic" | "slim"; dataUrl: string }>("skin_fetch_player", { name }),
  skinUpload: (id: string) => call<string>("skin_upload", { id }),
  skinApplyLocal: (id: string, profileId: string) => call<string>("skin_apply_local", { id, profileId }),
};
