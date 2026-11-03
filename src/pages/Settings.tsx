import { useEffect, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Copy, Download, FolderOpen, Gauge, KeyRound, Pickaxe, Search, SlidersHorizontal, Trash2, Wrench, X } from "lucide-react";
import { api, type JavaInstall, type LogFile, type Settings as S } from "../api";
import { AccountDialog } from "../components/AccountDialog";
import { Alert, Badge, Button, Confirm, Empty, Field, FieldSearch, Modal, Select, Toggle } from "../components/ui";
import { bytes } from "../format";
import { useStore } from "../store";

type Tab = "general" | "minecraft" | "performance" | "downloads" | "accounts" | "advanced";

const SECTIONS: { id: Tab; label: string; about: string; icon: typeof SlidersHorizontal }[] = [
  { id: "general", label: "General", about: "Theme, folders, official launcher", icon: SlidersHorizontal },
  { id: "minecraft", label: "Minecraft", about: "Java, memory, game window", icon: Pickaxe },
  { id: "performance", label: "Performance", about: "Presets and FPS mods", icon: Gauge },
  { id: "downloads", label: "Downloads", about: "Speed, retries, cache", icon: Download },
  { id: "accounts", label: "Accounts & services", about: "Microsoft, CurseForge", icon: KeyRound },
  { id: "advanced", label: "Advanced", about: "Logs, diagnostics, data", icon: Wrench },
];

export function Settings() {
  const { settings, saveSettings, subPage, info, run, toast, profiles } = useStore();
  // "search|java" opens the page searching (from Ctrl+K); otherwise the sub page is a section
  const [subTab, subQuery] = (subPage ?? "").split("|");
  const [tab, setTab] = useState<Tab>(SECTIONS.some((x) => x.id === subTab) ? (subTab as Tab) : "general");
  const [query, setQuery] = useState(subTab === "search" ? subQuery ?? "" : "");
  const q = query.trim().toLowerCase();
  const [s, setS] = useState<S | null>(settings);
  const [error, setError] = useState<string | null>(null);
  const [javas, setJavas] = useState<JavaInstall[]>([]);
  const [resetAsk, setResetAsk] = useState(false);
  useEffect(() => setS(settings), [settings]);
  useEffect(() => { api.listJava().then(setJavas).catch(() => undefined); }, []);
  if (!s) return null;
  const dirty = JSON.stringify(s) !== JSON.stringify(settings);
  const set = <K extends keyof S>(k: K, v: S[K]) => setS({ ...s, [k]: v });
  const save = async () => {
    setError(null);
    try {
      await saveSettings(s);
      toast("success", "Settings saved");
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  const pickDir = async (k: "downloadDir" | "minecraftDir") => {
    const d = await open({ directory: true, multiple: false });
    if (typeof d === "string") set(k, d);
  };
  const num = (v: string) => (v.trim() === "" ? null : Number(v));
  // one section at a time, or every section while searching (fields that don't match hide themselves)
  const sec = (id: Tab, body: ReactNode) =>
    q || tab === id ? (
      <section key={id} className="settings-section">
        <h2 className="settings-title">{SECTIONS.find((x) => x.id === id)?.label}</h2>
        {body}
      </section>
    ) : null;

  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Settings</h1><div className="sub">Saved to {info?.dataDir}\settings.json{info?.portable ? " (portable mode)" : ""}</div></div>
        <div className="actions">
          <Button variant="ghost" onClick={() => setResetAsk(true)}>Reset to defaults</Button>
        </div>
      </div>
      {error && <div className="mb"><Alert tone="error" title="Not saved">{error}</Alert></div>}
      <div className="settings">
      <nav className="settings-nav" aria-label="Settings sections">
        <label className="settings-search">
          <Search size={15} />
          <input placeholder="Search settings" value={query} onChange={(e) => setQuery(e.target.value)} aria-label="Search settings" />
          {query && <button className="btn ghost icon sm" onClick={() => setQuery("")} aria-label="Clear search"><X size={14} /></button>}
        </label>
        {SECTIONS.map((x) => (
          <button key={x.id} className={`settings-link ${!q && tab === x.id ? "active" : ""}`} onClick={() => { setQuery(""); setTab(x.id); }}>
            <x.icon size={17} />
            <span className="grow"><span className="settings-link-name">{x.label}</span><span className="settings-link-about">{x.about}</span></span>
          </button>
        ))}
      </nav>
      <div className={`settings-body ${q ? "searching" : ""}`}>
      <FieldSearch.Provider value={q}>
      {q && <div className="settings-noresults"><Empty title={`No setting matches “${query.trim()}”`} icon={<Search size={36} />}>Try another word, like “java”, “theme” or “memory”.</Empty></div>}

      {sec("general", (
        <div className="card form-grid">
          <Field label="Theme"><Select value={s.theme} onChange={(v) => set("theme", v)} options={[{ value: "dark", label: "Dark" }, { value: "light", label: "Light" }]} /></Field>
          <Field label="Accent colour"><div className="row"><input type="color" value={s.accent} onChange={(e) => set("accent", e.target.value)} style={{ width: 44, height: 34, border: "none", background: "none" }} /><input className="input grow mono" value={s.accent} onChange={(e) => set("accent", e.target.value)} /></div></Field>
          <Field label="Language" hint="More languages can be added later; English is the only translation today."><Select value={s.language} onChange={(v) => set("language", v)} options={[{ value: "en", label: "English" }]} /></Field>
          <Field label="On startup open"><Select value={s.startupPage} onChange={(v) => set("startupPage", v)} options={[{ value: "home", label: "Home" }, { value: "last", label: "The last page I used" }]} /></Field>
          <Field label="Notifications" hint="In-app messages for finished installs and downloads. Errors are always shown."><label className="row"><Toggle on={s.notifications} onChange={(v) => set("notifications", v)} label="Notifications" /> Show notifications</label></Field>
          <Field label="Check mod updates on startup" hint="One request to Modrinth for the selected profile when the launcher opens."><label className="row"><Toggle on={s.checkModUpdatesOnStartup} onChange={(v) => set("checkModUpdatesOnStartup", v)} label="Check updates" /> Enabled</label></Field>
          <Field label="Export / download folder" hint="Default location for exported profiles. Empty = your Downloads folder." full>
            <div className="row"><input className="input grow" value={s.downloadDir ?? ""} placeholder="Downloads" onChange={(e) => set("downloadDir", e.target.value || null)} /><Button onClick={() => pickDir("downloadDir")}>Browse…</Button></div>
          </Field>
          <Field label="Official Minecraft folder" hint="Where to look for the official launcher's installation. Empty = %APPDATA%\.minecraft." full>
            <div className="row"><input className="input grow" value={s.minecraftDir ?? ""} placeholder={info?.minecraftDir} onChange={(e) => set("minecraftDir", e.target.value || null)} /><Button onClick={() => pickDir("minecraftDir")}>Browse…</Button></div>
          </Field>
          <Field label="Official launcher" hint="When Shadow Client opens the Minecraft Launcher with a profile, that profile is the only installation next to PLAY. Your other installations are put aside, not deleted; turn this off and save to get them back." full>
            <label className="row"><Toggle on={s.officialOnlyThis} onChange={(v) => set("officialOnlyThis", v)} label="Only show the profile you open" /> Only show the profile you open</label>
          </Field>
        </div>
      ))}

      {sec("minecraft", (
        <div className="card form-grid">
          <Field label="Default loader for new profiles"><Select value={s.defaultLoader} onChange={(v) => set("defaultLoader", v)} options={[{ value: "fabric", label: "Fabric" }, { value: "quilt", label: "Quilt" }, { value: "vanilla", label: "Vanilla" }]} /></Field>
          <Field label="Default profile"><select className="select" value={s.defaultProfile ?? ""} onChange={(e) => set("defaultProfile", e.target.value || null)}><option value="">Last selected</option>{profiles.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}</select></Field>
          <Field label="Default version for new profiles" hint="Empty = newest release supported by Sodium"><input className="input" value={s.defaultVersion ?? ""} onChange={(e) => set("defaultVersion", e.target.value || null)} /></Field>
          <Field label="Java runtime" hint="Automatic picks the runtime each Minecraft version requires (recommended).">
            <select className="select" value={s.javaPath ?? ""} onChange={(e) => set("javaPath", e.target.value || null)}>
              <option value="">Automatic per version</option>
              {javas.map((j) => <option key={j.path} value={j.path}>Java {j.major} — {j.source}</option>)}
            </select>
          </Field>
          <Field label="Minimum memory (MB)" hint="Empty = automatic"><input className="input" type="number" value={s.memoryMinMb ?? ""} onChange={(e) => set("memoryMinMb", num(e.target.value))} /></Field>
          <Field label="Maximum memory (MB)" hint="Empty = recommended per profile. More memory ≠ more FPS."><input className="input" type="number" value={s.memoryMaxMb ?? ""} onChange={(e) => set("memoryMaxMb", num(e.target.value))} /></Field>
          <Field label="Garbage collector preset" hint={s.jvmPreset === "mojang" ? "G1 with a 50 ms pause target — what Mojang's launcher uses." : s.jvmPreset === "zgc" ? "Generational ZGC: shortest pauses, more memory/CPU. Needs Java 21+; falls back to G1 otherwise." : "Only heap flags; the JVM decides the rest."}>
            <Select value={s.jvmPreset} onChange={(v) => set("jvmPreset", v)} options={[{ value: "mojang", label: "Mojang defaults (G1)" }, { value: "minimal", label: "Minimal" }, { value: "zgc", label: "Generational ZGC (16 GB+ RAM)" }]} />
          </Field>
          <Field label="Extra JVM arguments" hint="Validated: only -XX:, -D, -Xss/-Xmn/-Xlog, --add-opens and similar. Agents, classpath and memory flags are rejected."><input className="input mono" value={s.jvmArgs} onChange={(e) => set("jvmArgs", e.target.value)} /></Field>
          <Field label="Game window width"><input className="input" type="number" value={s.resolutionWidth ?? ""} placeholder="854" onChange={(e) => set("resolutionWidth", num(e.target.value))} /></Field>
          <Field label="Game window height"><input className="input" type="number" value={s.resolutionHeight ?? ""} placeholder="480" onChange={(e) => set("resolutionHeight", num(e.target.value))} /></Field>
          <Field label="Fullscreen"><label className="row"><Toggle on={s.fullscreen} onChange={(v) => set("fullscreen", v)} label="Fullscreen" /> Start in fullscreen</label></Field>
          <Field label="When the game starts"><Select value={s.onGameLaunch} onChange={(v) => set("onGameLaunch", v)} options={[{ value: "minimize", label: "Minimize the launcher" }, { value: "hide", label: "Hide the launcher until the game closes" }, { value: "keep", label: "Keep the launcher open" }]} /></Field>
          <Field label="Version list"><div className="col"><label className="row"><Toggle on={s.showSnapshots} onChange={(v) => set("showSnapshots", v)} label="Snapshots" /> Show snapshots</label><label className="row"><Toggle on={s.showOldVersions} onChange={(v) => set("showOldVersions", v)} label="Old" /> Show old alpha/beta</label></div></Field>
          <Field label="Extra folders to scan for installations" hint="One folder per line (e.g. a second .minecraft or another launcher's instance)." full>
            <textarea className="textarea" rows={3} value={s.extraScanDirs.join("\n")} onChange={(e) => set("extraScanDirs", e.target.value.split("\n").map((x) => x.trim()).filter(Boolean))} />
          </Field>
        </div>
      ))}

      {sec("performance", (
        <div className="card form-grid">
          <Field label="Default preset for new profiles"><Select value={s.defaultPreset} onChange={(v) => set("defaultPreset", v)} options={[{ value: "auto", label: "Automatic (from benchmark)" }, { value: "low", label: "Low-end" }, { value: "balanced", label: "Balanced" }, { value: "high", label: "High FPS (PvP)" }, { value: "custom", label: "Custom" }]} /></Field>
          <Field label="Automatic Sodium" hint="Install Sodium + selected performance mods when creating Performance/PvP profiles (only compatible builds)."><label className="row"><Toggle on={s.autoInstallSodium} onChange={(v) => set("autoInstallSodium", v)} label="Auto Sodium" /> Enabled</label></Field>
          <Field label="Optional performance mods for new profiles" full>
            <div className="row wrap">
              {["sodium", "lithium", "ferrite-core", "immediatelyfast", "entityculling", "dynamic-fps", "modernfix", "moreculling", "badoptimizations"].map((slug) => (
                <label key={slug} className="row small" style={{ marginRight: 12 }}>
                  <input type="checkbox" checked={s.performanceMods.includes(slug)} onChange={(e) => set("performanceMods", e.target.checked ? [...s.performanceMods, slug] : s.performanceMods.filter((x) => x !== slug))} /> {slug}
                </label>
              ))}
            </div>
          </Field>
          <Field label="Custom preset values" hint="Render/simulation distance, particles, FPS limit, VSync and more are edited on the Performance page (with live warnings)." full>
            <div className="small dim">Render {s.customGraphics.renderDistance} · Simulation {s.customGraphics.simulationDistance} · Particles {["all", "decreased", "minimal"][s.customGraphics.particles]} · FPS {s.customGraphics.maxFps >= 260 ? "unlimited" : s.customGraphics.maxFps} · VSync {s.customGraphics.vsync ? "on" : "off"}</div>
          </Field>
        </div>
      ))}

      {sec("downloads", <DownloadSettings s={s} set={set} />)}
      {sec("accounts", <AccountsSettings s={s} set={set} />)}
      {sec("advanced", (
        <div className="col" style={{ gap: 14 }}>
          <div className="card form-grid">
            <Field label="Debug logging" hint="More detail in the launcher log (still redacted). Takes effect immediately."><label className="row"><Toggle on={s.debugLogging} onChange={(v) => set("debugLogging", v)} label="Debug logging" /> Enabled</label></Field>
            <Field label="Ask to benchmark on first run"><label className="row"><Toggle on={s.promptBenchmarkOnFirstRun} onChange={(v) => set("promptBenchmarkOnFirstRun", v)} label="Benchmark prompt" /> Enabled</label></Field>
            <Field label="Experimental: launch Forge/NeoForge installs" hint="Allows launching Forge/NeoForge versions that the official installer already set up in an existing folder. Not tested for every version."><label className="row"><Toggle on={s.experimentalForgeLaunch} onChange={(v) => set("experimentalForgeLaunch", v)} label="Experimental Forge" /> Enabled</label></Field>
          </div>
          {!q && <Diagnostics />}
          {!q && (
            <div className="card row wrap">
              <span className="grow dim">Data folder: <code>{info?.dataDir}</code></span>
              <Button icon={<FolderOpen size={14} />} onClick={() => run(() => api.openDataDir())}>Open data folder</Button>
            </div>
          )}
        </div>
      ))}
      </FieldSearch.Provider>
      </div>
      </div>
      {dirty && (
        <div className="savebar" role="status">
          <span className="grow">You have unsaved changes</span>
          <Button variant="ghost" onClick={() => setS(settings)}>Discard</Button>
          <Button variant="primary" onClick={save}>Save changes</Button>
        </div>
      )}
      {resetAsk && <Confirm title="Reset all settings?" confirmLabel="Reset" danger onCancel={() => setResetAsk(false)} onConfirm={async () => { await run(() => api.resetSettings().then((r) => saveSettings(r)), "Settings reset"); setResetAsk(false); }}>Profiles, mods, accounts and worlds are not affected.</Confirm>}
    </div>
  );
}

function DownloadSettings({ s, set }: { s: S; set: <K extends keyof S>(k: K, v: S[K]) => void }) {
  const { run } = useStore();
  const [cache, setCache] = useState<{ httpCacheBytes: number; tmpBytes: number } | null>(null);
  useEffect(() => { api.cacheInfo().then(setCache); }, []);
  return (
    <div className="card form-grid">
      <Field label={`Parallel downloads: ${s.downloadConcurrency}`} hint="Lower this on slow connections."><input type="range" min={1} max={16} value={s.downloadConcurrency} onChange={(e) => set("downloadConcurrency", Number(e.target.value))} /></Field>
      <Field label={`Retries per file: ${s.downloadRetries}`}><input type="range" min={0} max={10} value={s.downloadRetries} onChange={(e) => set("downloadRetries", Number(e.target.value))} /></Field>
      <Field label="API cache lifetime (minutes)" hint="Search results and metadata are cached to stay within Modrinth/CurseForge rate limits."><input className="input" type="number" min={1} max={1440} value={s.cacheTtlMinutes} onChange={(e) => set("cacheTtlMinutes", Number(e.target.value))} /></Field>
      <Field label="Cache">
        <div className="row"><span className="grow small dim">{cache ? `${bytes(cache.httpCacheBytes)} API cache · ${bytes(cache.tmpBytes)} temporary files` : "…"}</span><Button size="sm" icon={<Trash2 size={14} />} onClick={async () => { const f = await run(() => api.clearCache()); if (f !== undefined) setCache(await api.cacheInfo()); }}>Clear</Button></div>
      </Field>
      <p className="small faint full">Game files (libraries, assets) are stored once and shared by all profiles; already-verified files are never downloaded again.</p>
    </div>
  );
}

function AccountsSettings({ s, set }: { s: S; set: <K extends keyof S>(k: K, v: S[K]) => void }) {
  const { run } = useStore();
  const [cf, setCf] = useState<boolean | null>(null);
  const [key, setKey] = useState("");
  const [accounts, setAccounts] = useState(false);
  useEffect(() => { api.curseforgeStatus().then(setCf); }, []);
  return (
    <div className="col" style={{ gap: 14 }}>
      <div className="card">
        <div className="card-head"><h3>Minecraft accounts</h3><div className="actions"><Button onClick={() => setAccounts(true)}>Manage accounts</Button></div></div>
        <Field label="Azure application (client) ID for Microsoft sign-in" hint={<>Microsoft requires every launcher to use its own Azure app approved for Minecraft. See docs/AUTH_SETUP.md. The ID is public, not a secret. Save settings after editing.</>}>
          <input className="input mono" value={s.msaClientId ?? ""} placeholder="00000000-0000-0000-0000-000000000000" onChange={(e) => set("msaClientId", e.target.value || null)} />
        </Field>
      </div>
      <div className="card">
        <div className="card-head"><h3>CurseForge API key</h3>{cf ? <Badge tone="accent">Configured</Badge> : <Badge>Not configured</Badge>}</div>
        <p className="small dim mb">Needed to search and download from CurseForge. Get a key at console.curseforge.com. It is encrypted with Windows DPAPI for your user account, sent only to api.curseforge.com, never logged and never shown again.</p>
        <div className="row">
          <input className="input grow mono" type="password" autoComplete="off" value={key} placeholder={cf ? "•••••••• (stored)" : "Paste your key"} onChange={(e) => setKey(e.target.value)} />
          <Button variant="primary" icon={<KeyRound size={14} />} disabled={key.length < 20} onClick={async () => { await run(() => api.setCurseforgeKey(key), "Key saved securely"); setKey(""); setCf(await api.curseforgeStatus()); }}>Save key</Button>
          {cf && <Button variant="danger" onClick={async () => { await run(() => api.clearCurseforgeKey(), "Key removed"); setCf(await api.curseforgeStatus()); }}>Remove</Button>}
        </div>
      </div>
      <div className="card">
        <h3>Modrinth</h3>
        <Field label="Contact for the User-Agent (optional)" hint="Modrinth asks API clients to identify themselves. An email or URL here is added to the User-Agent header sent to Modrinth. Leave empty to send only the app name and version.">
          <input className="input" value={s.modrinthContact ?? ""} onChange={(e) => set("modrinthContact", e.target.value || null)} />
        </Field>
      </div>
      {accounts && <AccountDialog onClose={() => setAccounts(false)} />}
    </div>
  );
}

function Diagnostics() {
  const { run, toast } = useStore();
  const [logs, setLogs] = useState<LogFile[]>([]);
  const [view, setView] = useState<{ name: string; text: string } | null>(null);
  const [report, setReport] = useState<string | null>(null);
  useEffect(() => { api.logsList().then(setLogs); }, []);
  return (
    <div className="card">
      <div className="card-head"><h3>Diagnostics &amp; logs</h3><div className="actions"><Button size="sm" onClick={async () => setReport((await run(() => api.diagnosticsReport())) ?? null)}>Create diagnostics report</Button></div></div>
      <p className="small dim mb">Logs never contain tokens; the viewer redacts again and the shareable report also replaces your Windows user name. Nothing is uploaded — you decide what to share.</p>
      <div className="list">
        {logs.slice(0, 12).map((l) => (
          <div className="list-row" key={l.path} style={{ minHeight: 40 }}>
            <span className="grow small">{l.kind === "game" ? "Game" : "Launcher"} · {l.name}</span>
            <span className="small faint">{bytes(l.size)}</span>
            <Button size="sm" onClick={async () => { const t = await run(() => api.logRead(l.path, false)); if (t !== undefined) setView({ name: l.name, text: t }); }}>View</Button>
          </div>
        ))}
        {logs.length === 0 && <div className="list-row dim">No logs yet.</div>}
      </div>
      {view && <Modal wide title={view.name} onClose={() => setView(null)}><div className="logbox">{view.text.slice(-200_000)}</div></Modal>}
      {report && (
        <Modal wide title="Diagnostics report (redacted)" onClose={() => setReport(null)} footer={<Button variant="primary" icon={<Copy size={14} />} onClick={async () => { await navigator.clipboard.writeText(report); toast("success", "Copied to clipboard"); }}>Copy</Button>}>
          <p className="small dim mb">Review before sharing. Tokens, keys and your user name are removed.</p>
          <div className="logbox">{report}</div>
        </Modal>
      )}
    </div>
  );
}
