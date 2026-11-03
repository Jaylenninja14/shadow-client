import { useCallback, useEffect, useState } from "react";
import { Activity, Boxes, Camera, Cpu, Download, FolderOpen, MemoryStick, Play, RefreshCw, Server, Sparkles, Square, UserRound } from "lucide-react";
import { api, type Preflight, type Problem } from "../api";
import { AccountDialog } from "../components/AccountDialog";
import { NewProfileDialog } from "../components/NewProfileDialog";
import { Alert, Badge, Button, Empty, Progress, Spinner } from "../components/ui";
import { bytes, duration, loaderLabel, percent, presetLabel, relativeTime } from "../format";
import { useStore } from "../store";

const STAGES: Record<string, string> = { validate: "Checking profile", files: "Verifying game files", java: "Selecting Java", auth: "Checking account", config: "Applying settings", launch: "Starting Minecraft" };

export function Home() {
  const { selected, profiles, selectProfile, games, stats, launchProgress, jobs, activeAccount, run, setPage, refreshProfiles } = useStore();
  const [pre, setPre] = useState<Preflight | null>(null);
  const [checking, setChecking] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [accounts, setAccounts] = useState(false);
  const [newProfile, setNewProfile] = useState(false);
  const [log, setLog] = useState<string[] | null>(null);

  const game = selected ? games[selected.id] : undefined;
  const running = game?.state === "running" || game?.state === "starting";
  const sample = selected ? stats[selected.id] : undefined;
  const progress = selected ? launchProgress[selected.id] : undefined;

  const check = useCallback(async () => {
    if (!selected) return;
    setChecking(true);
    try {
      setPre(await api.preflight(selected.id));
    } catch (e) {
      setPre(null);
      setLaunchError(e instanceof Error ? e.message : String(e));
    } finally {
      setChecking(false);
    }
  }, [selected]);

  useEffect(() => {
    setLaunchError(null);
    check();
    // re-check when the profile or account changes
  }, [selected?.id, activeAccount?.id, check]);

  // Game log only while the user is looking at it (1 s refresh, stops when hidden).
  useEffect(() => {
    if (log === null || !selected || !running) return;
    const t = window.setInterval(() => api.gameLogTail(selected.id).then(setLog).catch(() => undefined), 1000);
    return () => window.clearInterval(t);
  }, [log === null, selected, running]);

  const play = async (force: boolean) => {
    if (!selected) return;
    setLaunching(true);
    setLaunchError(null);
    try {
      await api.play(selected.id, force);
    } catch (e) {
      setLaunchError(e instanceof Error ? e.message : String(e));
    } finally {
      setLaunching(false);
      refreshProfiles();
      check();
    }
  };

  const fix = async (p: Problem) => {
    if (!p.action) return;
    if (p.action.startsWith("download-java:")) {
      await run(() => api.downloadJava(p.action!.split(":")[1]), "Java runtime installed and verified");
      check();
    } else if (p.action === "open-mods") setPage("mods");
    else if (p.action === "choose-account") setAccounts(true);
    else if (p.action === "open-settings") setPage("settings");
  };

  if (profiles.length === 0) {
    return (
      <div className="page">
        <Empty title="No profiles yet" icon={<Play size={40} />}>
          <p className="dim">Create an optimized profile (Fabric + Sodium) or add an existing installation.</p>
          <div className="row mt" style={{ justifyContent: "center" }}>
            <Button variant="primary" onClick={() => setNewProfile(true)}>Create profile</Button>
            <Button onClick={() => setPage("profiles", "import")}>Add existing installation</Button>
          </div>
        </Empty>
        {newProfile && <NewProfileDialog onClose={() => { setNewProfile(false); refreshProfiles(); }} />}
      </div>
    );
  }
  if (!selected) return null;

  const errors = pre?.problems.filter((p) => p.severity === "error") ?? [];
  const onlyModErrors = errors.length > 0 && errors.every((p) => p.action === "open-mods");
  const ready = pre?.ready && !running;
  const activeJobs = jobs.filter((j) => j.status === "running");
  const statusText = running ? (game?.state === "starting" ? "Starting…" : "Running") : checking ? "Checking…" : pre ? (pre.ready ? "Ready to launch" : `${errors.length} problem${errors.length === 1 ? "" : "s"} to fix`) : "Not checked";
  const dot = running ? "ok" : pre?.ready ? "ok" : errors.length ? "err" : "warn";

  return (
    <div className="page">
      <div className="hero art home-hero">
        <div>
          <div className="hero-pill"><span className={`status-dot ${dot}`} /> {statusText}</div>
          <div className="hero-title">{selected.name}</div>
          <div className="row wrap">
            <Badge tone="accent">Minecraft {selected.mcVersion}</Badge>
            <Badge>{loaderLabel(selected.loader)}{selected.loaderVersion ? ` ${selected.loaderVersion}` : ""}</Badge>
            <Badge>Preset: {presetLabel(selected.preset)}</Badge>
            {selected.external && <Badge tone="info">Existing installation</Badge>}
          </div>
        </div>
        <div className="hero-side">
          {running ? (
            <Button className="play-btn" variant="danger" icon={<Square size={18} />} onClick={() => run(() => api.killGame(selected.id))}>Stop</Button>
          ) : (
            <Button className="play-btn" variant="primary" disabled={launching || checking || !ready} icon={<Play size={20} />} onClick={() => play(false)}>
              {launching ? "Launching" : "Play"}
            </Button>
          )}
          {!running && onlyModErrors && <Button size="sm" variant="ghost" onClick={() => play(true)}>Launch anyway</Button>}
          <div className="row small dim">
            <UserRound size={14} />
            {activeAccount ? `${activeAccount.username}${activeAccount.kind === "offline" ? " (offline)" : ""}` : "No account"}
            <a onClick={() => setAccounts(true)}>Change</a>
          </div>
        </div>
        <div className="hero-stats">
          <div className="stat"><span className="v">{pre ? pre.enabledModCount : "—"}</span><span className="l">Mods enabled</span></div>
          <div className="stat"><span className="v">{relativeTime(selected.lastPlayed)}</span><span className="l">Last played</span></div>
          <div className="stat"><span className="v">{duration(selected.totalPlaySecs)}</span><span className="l">Play time</span></div>
          <div className="stat"><span className="v">{pre?.java ? `Java ${pre.java.major}` : "—"}</span><span className="l">Runtime</span></div>
          <div className="stat"><span className="v">{pre ? `${(pre.memoryMaxMb / 1024).toFixed(1)} GB` : "—"}</span><span className="l">Max memory</span></div>
        </div>
        {(launching || progress) && !running && (
          <div className="hero-progress">
            <div className="row small dim mb"><Spinner /> {progress ? STAGES[progress.stage] ?? progress.message : "Preparing"} — {progress?.message}</div>
            {activeJobs.length > 0 && activeJobs.slice(0, 1).map((j) => (
              <div key={j.id}>
                <Progress value={j.totalBytes ? percent(j.downloadedBytes, j.totalBytes) : percent(j.doneFiles, j.totalFiles)} />
                <div className="small faint mt">{j.title}: {j.doneFiles}/{j.totalFiles} files · {bytes(j.downloadedBytes)}{j.totalBytes ? ` of ${bytes(j.totalBytes)}` : ""} · {bytes(j.bytesPerSec)}/s</div>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="quick">
        <button className="quick-btn" onClick={() => setPage("mods")}><span className="qi"><Boxes size={17} /></span><span className="ellipsis">Mods<span className="qs">{pre ? `${pre.enabledModCount} enabled` : "Browse and manage"}</span></span></button>
        <button className="quick-btn" onClick={() => setPage("features")}><span className="qi"><Sparkles size={17} /></span><span className="ellipsis">Features<span className="qs">Mod menu and HUD</span></span></button>
        <button className="quick-btn" onClick={() => setPage("servers")}><span className="qi"><Server size={17} /></span><span className="ellipsis">Servers<span className="qs">Ping and join</span></span></button>
        <button className="quick-btn" onClick={() => setPage("screenshots")}><span className="qi"><Camera size={17} /></span><span className="ellipsis">Screenshots<span className="qs">This profile</span></span></button>
        <button className="quick-btn" onClick={() => run(() => api.openProfileFolder(selected.id))}><span className="qi"><FolderOpen size={17} /></span><span className="ellipsis">Game folder<span className="qs">Open in Explorer</span></span></button>
      </div>

      {launchError && <div className="mt"><Alert tone="error" title="Launch failed">{launchError}</Alert></div>}
      {game?.state === "crashed" && (
        <div className="mt">
          <Alert tone="error" title="Minecraft crashed" action={game.crashReport ? <Button size="sm" onClick={() => run(() => api.openPath(game.crashReport!))}>Open crash report</Button> : undefined}>{game.message}</Alert>
        </div>
      )}

      <div className="grid cols-2 mt">
        <div className="card">
          <div className="card-head">
            <h3>Pre-launch check</h3>
            <div className="actions"><Button size="sm" variant="ghost" icon={<RefreshCw size={14} />} onClick={check}>Re-check</Button></div>
          </div>
          {checking && !pre && <div className="row dim"><Spinner /> Checking…</div>}
          {pre && pre.problems.length === 0 && <Alert tone="success" title="Everything looks good">Files, mods, Java, account and memory are ready.</Alert>}
          {pre?.problems.map((p, i) => (
            <Alert key={i} tone={p.severity} title={p.title} action={p.action ? <Button size="sm" onClick={() => fix(p)}>{p.action.startsWith("download-java") ? "Download Java" : p.action === "open-mods" ? "Open mods" : p.action === "choose-account" ? "Add account" : "Open settings"}</Button> : undefined}>
              {p.detail}
            </Alert>
          ))}
          {pre && <div className="small faint mt">{pre.memoryReasons.join(" · ")}</div>}
        </div>

        <div className="card">
          <div className="card-head"><h3>{running ? "Game process" : "Last session"}</h3></div>
          {running ? (
            <div className="col">
              <div className="row wrap" style={{ gap: 28 }}>
                <div className="stat"><span className="v"><MemoryStick size={14} /> {sample ? `${sample.memoryMb} MB` : "—"}</span><span className="l">Game RAM (measured)</span></div>
                <div className="stat"><span className="v"><Cpu size={14} /> {sample ? `${sample.cpuPercent.toFixed(0)}%` : "—"}</span><span className="l">Game CPU (measured)</span></div>
                <div className="stat"><span className="v"><Activity size={14} /> F3</span><span className="l">In-game FPS</span></div>
              </div>
              <p className="small faint">FPS is only visible inside the game (press F3); the launcher can't read it, so it doesn't guess.</p>
              <div className="row">
                <Button size="sm" onClick={async () => setLog(await api.gameLogTail(selected.id))}>{log ? "Refresh log" : "Show game log"}</Button>
                {log && <Button size="sm" variant="ghost" onClick={() => setLog(null)}>Hide log</Button>}
              </div>
              {log && <div className="logbox">{log.slice(-200).join("\n")}</div>}
            </div>
          ) : selected.lastLaunch ? (
            <div className="kv">
              <span>Result</span><span>{selected.lastLaunch.result === "ok" ? "Closed normally" : selected.lastLaunch.result}</span>
              <span>When</span><span>{relativeTime(selected.lastLaunch.at)}</span>
              {selected.lastLaunch.durationSecs != null && <><span>Duration</span><span>{duration(selected.lastLaunch.durationSecs)}</span></>}
              {selected.lastLaunch.message && <><span>Details</span><span style={{ whiteSpace: "pre-wrap" }}>{selected.lastLaunch.message}</span></>}
            </div>
          ) : (
            <p className="dim">This profile hasn't been played yet.</p>
          )}
          <div className="row mt">
            <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openProfileFolder(selected.id))}>Open game folder</Button>
          </div>
        </div>

        <div className="card">
          <div className="card-head"><h3>Recent profiles</h3><div className="actions"><Button size="sm" variant="ghost" onClick={() => setPage("profiles")}>All profiles</Button></div></div>
          <div className="list">
            {profiles.slice(0, 5).map((p) => (
              <div key={p.id} className="list-row" style={{ cursor: "pointer", minHeight: 44 }} onClick={() => selectProfile(p.id)}>
                <div className="grow">
                  <strong>{p.name}</strong> <span className="small faint">{p.mcVersion} · {loaderLabel(p.loader)}</span>
                </div>
                {games[p.id]?.state === "running" && <Badge tone="accent">Running</Badge>}
                {p.id === selected.id ? <Badge tone="accent">Selected</Badge> : <span className="small faint">{relativeTime(p.lastPlayed)}</span>}
              </div>
            ))}
          </div>
        </div>

        <div className="card">
          <div className="card-head"><h3>Downloads</h3><div className="actions"><Button size="sm" variant="ghost" icon={<Download size={14} />} onClick={() => setPage("downloads")}>Open</Button></div></div>
          {activeJobs.length === 0 && <p className="dim">No active downloads.{jobs[0] ? ` Last: ${jobs[0].title} (${jobs[0].status}).` : ""}</p>}
          {activeJobs.map((j) => (
            <div key={j.id} className="mb">
              <div className="row small"><span className="grow ellipsis">{j.title}</span><span className="faint">{bytes(j.bytesPerSec)}/s</span></div>
              <Progress value={j.totalBytes ? percent(j.downloadedBytes, j.totalBytes) : percent(j.doneFiles, j.totalFiles)} />
            </div>
          ))}
        </div>
      </div>
      {accounts && <AccountDialog onClose={() => { setAccounts(false); check(); }} />}
    </div>
  );
}
