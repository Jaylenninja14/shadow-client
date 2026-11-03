import { useEffect, useState } from "react";
import { FolderSearch, Gauge, Rocket } from "lucide-react";
import { api, type BenchmarkResult, type DetectedInstall, type JavaInstall, type OfficialLauncher } from "../api";
import { branding } from "../branding";
import { presetLabel } from "../format";
import { useStore } from "../store";
import { NewProfileDialog } from "./NewProfileDialog";
import { Alert, Badge, Button, Modal, Spinner } from "./ui";

/** First-run flow: show detected installs, offer choices A–D, ask before benchmarking. */
export function Onboarding({ onClose }: { onClose: () => void }) {
  const { patchAppState, refreshProfiles, run, settings } = useStore();
  const [step, setStep] = useState<"scan" | "pick" | "bench" | "done">("scan");
  const [installs, setInstalls] = useState<DetectedInstall[] | null>(null);
  const [javas, setJavas] = useState<JavaInstall[]>([]);
  const [official, setOfficial] = useState<OfficialLauncher | null>(null);
  const [mode, setMode] = useState<"manage" | "copy" | null>(null);
  const [newProfile, setNewProfile] = useState(false);
  const [bench, setBench] = useState<BenchmarkResult | null>(null);
  const [benchRunning, setBenchRunning] = useState(false);

  useEffect(() => {
    Promise.all([api.detectInstalls(), api.listJava(), api.officialLauncher()])
      .then(([i, j, o]) => {
        setInstalls(i);
        setJavas(j);
        setOfficial(o);
      })
      .catch(() => setInstalls([]));
  }, []);

  const finish = async (choice: string) => {
    await patchAppState({ onboardingDone: true, onboardingChoice: choice });
    await refreshProfiles();
    if (settings?.promptBenchmarkOnFirstRun) setStep("bench");
    else onClose();
  };

  const totals = (i: DetectedInstall) => ({
    mods: i.gameDirs.reduce((n, g) => n + g.mods.length, 0),
    worlds: i.gameDirs.reduce((n, g) => n + g.worlds.length, 0),
    packs: i.gameDirs.reduce((n, g) => n + g.resourcepacks.length, 0),
    shaders: i.gameDirs.reduce((n, g) => n + g.shaderpacks.length, 0),
    screenshots: i.gameDirs.reduce((n, g) => n + g.screenshots, 0),
  });

  if (newProfile) return <NewProfileDialog initialKind="performance" onClose={async () => { setNewProfile(false); await finish("optimized"); }} />;

  if (step === "bench") {
    return (
      <Modal title="Recommend settings for this PC?" onClose={onClose} footer={<>
        <Button onClick={onClose}>{bench ? "Done" : "Skip"}</Button>
        {!bench && <Button variant="primary" disabled={benchRunning} onClick={async () => { setBenchRunning(true); const r = await run(() => api.runBenchmark()); setBenchRunning(false); if (r) setBench(r); }}>{benchRunning ? "Measuring…" : "Run 5-second benchmark"}</Button>}
      </>}>
        <p className="dim">The benchmark measures CPU, memory and disk speed for a few seconds and combines that with your detected hardware to suggest a graphics preset. It does not measure in-game FPS, uploads nothing, and only runs when you ask (you can re-run it later on the Performance page).</p>
        {benchRunning && <div className="row mt"><Spinner /> Measuring…</div>}
        {bench && (
          <div className="mt">
            <Alert tone="success" title={`Recommended preset: ${presetLabel(bench.recommendedPreset)}`}>{bench.reasons.join("\n")}</Alert>
          </div>
        )}
      </Modal>
    );
  }

  if (step === "pick" && mode && installs) {
    const candidates = installs.flatMap((inst) =>
      inst.launcherProfiles.length > 0
        ? inst.launcherProfiles.map((lp) => ({ inst, label: `${inst.label} — ${lp.name}`, gameDir: lp.resolvedGameDir, versionId: lp.lastVersionId, loader: null as string | null, mc: null as string | null }))
        : inst.gameDirs.slice(0, 1).map((g) => ({ inst, label: inst.label, gameDir: g.path, versionId: null as string | null, loader: inst.instanceLoader, mc: inst.instanceVersion })),
    );
    return (
      <Modal wide title={mode === "manage" ? "Manage an existing installation" : "Bring over an existing profile"} onClose={() => setStep("scan")}>
        <p className="dim mb">
          {mode === "manage"
            ? "The profile will use this folder as-is. Nothing in it is changed until you install, update or remove something yourself, and replaced mods are always backed up first."
            : "Pick one: a new Shadow Client profile with the same name is made, and its mods, configs and resource packs are copied into it. Worlds stay where they are, and the original folder isn't changed."}
        </p>
        <div className="list">
          {candidates.map((c, i) => (
            <div className="list-row" key={i}>
              <div className="grow">
                <strong>{c.label}</strong>
                <div className="small faint ellipsis">{c.gameDir}</div>
                <div className="small dim">{c.versionId ? `Version: ${c.versionId}` : c.mc ? `Minecraft ${c.mc} ${c.loader ?? ""}` : ""}</div>
              </div>
              <Button size="sm" variant="primary" onClick={async () => {
                const p = await run(() => api.importInstall({ root: c.inst.root, gameDir: c.gameDir, versionId: c.versionId, mcVersion: c.mc, loader: c.loader, name: c.label.replace(/^.*— /, "").slice(0, 40) || "Imported", mode, includeWorlds: false }), "Profile added");
                if (p) await finish(mode === "manage" ? "manage" : "modded");
              }}>Copy into Shadow Client</Button>
            </div>
          ))}
          {candidates.length === 0 && <div className="list-row dim">No installations found. Add a folder under Settings → Minecraft → Scan folders.</div>}
        </div>
      </Modal>
    );
  }

  return (
    <Modal wide title={`Welcome to ${branding.productName}`} onClose={async () => { await patchAppState({ onboardingDone: true }); onClose(); }}>
      <p className="dim">{branding.tagline}. No account with us is needed: profiles, mods and settings stay on this PC.</p>
      <h3 className="mt mb">What we found</h3>
      {!installs && <div className="row dim"><Spinner /> Scanning standard Minecraft locations (read-only)…</div>}
      {installs && (
        <div className="col">
          {installs.length === 0 && <Alert tone="info">No existing Minecraft installation was found. You can start fresh.</Alert>}
          {installs.filter((i) => i.source === "official" || i.source === "custom").map((i) => {
            const t = totals(i);
            return (
              <div className="card" key={i.root} style={{ padding: 12 }}>
                <div className="row wrap">
                  <FolderSearch size={18} />
                  <strong>{i.label}</strong>
                  <span className="small faint ellipsis grow">{i.root}</span>
                </div>
                <div className="row wrap mt small dim">
                  {i.launcherProfiles.length > 0 && <Badge>{i.launcherProfiles.length} launcher profiles</Badge>}
                  {i.versions.length > 0 && <Badge>{i.versions.length} versions{i.versions.some((v) => v.loader !== "vanilla") ? ` (${[...new Set(i.versions.filter((v) => v.loader !== "vanilla").map((v) => v.loader))].join(", ")})` : ""}</Badge>}
                  <Badge>{t.mods} mods</Badge>
                  <Badge>{t.worlds} worlds</Badge>
                  <Badge>{t.packs} resource packs</Badge>
                  <Badge>{t.shaders} shader packs</Badge>
                  <Badge>{t.screenshots} screenshots</Badge>
                </div>
                {i.notes.map((n, k) => <div key={k} className="small faint mt">{n}</div>)}
              </div>
            );
          })}
          {(() => {
            const others = installs.filter((i) => i.source !== "official" && i.source !== "custom");
            if (others.length === 0) return null;
            const bySource = others.reduce<Record<string, DetectedInstall[]>>((acc, i) => ({ ...acc, [i.source]: [...(acc[i.source] ?? []), i] }), {});
            const names: Record<string, string> = { curseforge: "CurseForge", prism: "Prism Launcher", multimc: "MultiMC", "modrinth-app": "Modrinth App" };
            return (
              <details className="card" style={{ padding: 12 }}>
                <summary style={{ cursor: "pointer" }}>
                  <strong>Other launchers:</strong> {Object.entries(bySource).map(([s, l]) => `${l.length} ${names[s] ?? s} instance${l.length === 1 ? "" : "s"}`).join(", ")}
                  <span className="small faint"> · {others.reduce((n, i) => n + totals(i).mods, 0)} mods, {others.reduce((n, i) => n + totals(i).worlds, 0)} worlds in total</span>
                </summary>
                <div className="col mt" style={{ maxHeight: 220, overflow: "auto", gap: 4 }}>
                  {others.map((i) => {
                    const t = totals(i);
                    return (
                      <div key={i.root} className="row small">
                        <span className="grow ellipsis">{i.label}</span>
                        <span className="faint">{i.instanceVersion ?? ""} {i.instanceLoader ?? ""} · {t.mods} mods · {t.worlds} worlds</span>
                      </div>
                    );
                  })}
                </div>
              </details>
            );
          })()}
          <div className="small dim">Java runtimes found: {javas.length === 0 ? "none (the official runtime is downloaded when needed)" : javas.map((j) => `Java ${j.major} (${j.source})`).join(", ")}</div>
          <div className="small dim">Official launcher: {official?.installed ? (official.kind === "store" ? "Microsoft Store edition installed" : "installed") : "not installed"}</div>
        </div>
      )}
      <h3 className="mt mb">How do you want to start?</h3>
      <div className="grid cols-2">
        <button className="choice" disabled={!installs?.length} onClick={() => { setMode("copy"); setStep("pick"); }}>
          <span className="letter">A</span>
          <span><strong>Bring over an existing profile</strong><br /><span className="small dim">Its mods, configs and resource packs are copied into a new Shadow Client profile with the same name. The original isn't changed.</span></span>
        </button>
        <button className="choice" onClick={() => setNewProfile(true)}>
          <span className="letter">B</span>
          <span><strong>Create a separate optimized profile</strong><br /><span className="small dim">Recommended. Fabric + Sodium and compatible performance mods in an isolated folder.</span></span>
        </button>
        <button className="choice" onClick={async () => { await run(() => api.openOfficialLauncher()); await finish("official"); }}>
          <span className="letter">C</span>
          <span><strong>Open the official Minecraft Launcher</strong><br /><span className="small dim">Keep using it; you can come back any time.</span></span>
        </button>
      </div>
      <div className="row mt small faint"><Rocket size={14} /> You can add more profiles later. <Gauge size={14} /> Settings can be changed any time.</div>
    </Modal>
  );
}
