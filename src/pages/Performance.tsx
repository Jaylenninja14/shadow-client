import { useCallback, useEffect, useState } from "react";
import { Cpu, Gauge, HardDrive, MemoryStick, Monitor, RefreshCw, RotateCcw } from "lucide-react";
import { api, type ApplyReport, type GraphicsSettings, type HardwareInfo, type MemoryRecommendation, type OptionsBackup, type PresetInfo, type PresetPreview } from "../api";
import { Alert, Badge, Button, Field, Modal, Select, Spinner, Toggle } from "../components/ui";
import { presetLabel, relativeTime } from "../format";
import { useStore } from "../store";

export function Performance() {
  const { selected, appState, stats, games, run, settings, saveSettings, refreshAccounts, refreshProfiles } = useStore();
  const [hw, setHw] = useState<HardwareInfo | null>(null);
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [benchRunning, setBenchRunning] = useState(false);
  const [preview, setPreview] = useState<{ id: string; data: PresetPreview } | null>(null);
  const [applied, setApplied] = useState<ApplyReport | null>(null);
  const [backups, setBackups] = useState<OptionsBackup[]>([]);
  const [current, setCurrent] = useState<Record<string, string> | null>(null);
  const [mem, setMem] = useState<MemoryRecommendation | null>(null);
  const [custom, setCustom] = useState<GraphicsSettings | null>(null);
  const [customWarn, setCustomWarn] = useState<string[]>([]);

  const loadProfileBits = useCallback(async () => {
    if (!selected) return;
    setBackups(await api.optionsBackups(selected.id));
    setCurrent(await api.currentGraphics(selected.id));
    setMem(await api.memoryRecommendation(selected.id));
  }, [selected]);

  useEffect(() => {
    api.hardwareInfo(false).then(setHw).catch(() => undefined);
    api.presetList().then(setPresets);
  }, []);
  useEffect(() => { loadProfileBits(); }, [loadProfileBits]);
  useEffect(() => { if (settings && !custom) setCustom(settings.customGraphics); }, [settings, custom]);
  useEffect(() => {
    if (!custom) return;
    const t = window.setTimeout(() => api.graphicsWarnings(custom).then(setCustomWarn).catch((e) => setCustomWarn([String(e.message ?? e)])), 250);
    return () => window.clearTimeout(t);
  }, [custom]);

  const bench = appState?.benchmark;
  const running = selected ? games[selected.id]?.state === "running" : false;
  const sample = selected ? stats[selected.id] : undefined;

  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Performance</h1><div className="sub">Real hardware data, an optional system benchmark, and presets that write actual Minecraft settings.</div></div>
      </div>

      <div className="grid cols-2">
        <div className="card">
          <div className="card-head"><h3>This PC</h3><div className="actions"><Button size="sm" variant="ghost" icon={<RefreshCw size={14} />} onClick={async () => setHw(await api.hardwareInfo(true))}>Refresh</Button></div></div>
          {!hw ? <Spinner /> : (
            <div className="kv">
              <span><Cpu size={13} /> CPU</span><span>{hw.cpuName} · {hw.cpuPhysicalCores ?? "?"} cores / {hw.cpuLogicalCores} threads{hw.cpuMhz ? ` · ${hw.cpuMhz} MHz` : ""}</span>
              <span><MemoryStick size={13} /> RAM</span><span>{(hw.ramTotalMb / 1024).toFixed(1)} GB total · {(hw.ramAvailableMb / 1024).toFixed(1)} GB free now</span>
              {hw.gpus.length === 0 && <><span><Monitor size={13} /> GPU</span><span>Not detected</span></>}
              {hw.gpus.map((g) => (
                <span key={g.name} style={{ display: "contents" }}>
                  <span><Monitor size={13} /> GPU</span>
                  <span>{g.name} <Badge tone={g.integrated ? "warning" : "accent"}>{g.integrated ? "Integrated" : "Dedicated"}</Badge>{g.vramMb ? ` · ${(g.vramMb / 1024).toFixed(1)} GB` : ""}{g.driverVersion ? ` · driver ${g.driverVersion}` : ""}</span>
                </span>
              ))}
              <span>Display</span><span>{hw.display ? `${hw.display.width}×${hw.display.height}${hw.display.refreshHz ? ` @ ${hw.display.refreshHz} Hz` : ""}` : "Unknown"}</span>
              <span>OS</span><span>{hw.osName ?? "Unknown"}</span>
              <span><HardDrive size={13} /> Free disk</span><span>{hw.freeDiskMb != null ? `${(hw.freeDiskMb / 1024).toFixed(1)} GB (launcher drive)` : "Unknown"}</span>
            </div>
          )}
        </div>

        <div className="card">
          <div className="card-head"><h3>System benchmark</h3><div className="actions">
            <Button size="sm" variant={bench ? "default" : "primary"} icon={<Gauge size={14} />} disabled={benchRunning} onClick={async () => { setBenchRunning(true); const r = await run(() => api.runBenchmark()); setBenchRunning(false); if (r) { setHw(r.hardware); await refreshAccounts(); } }}>{benchRunning ? "Measuring…" : bench ? "Run again" : "Run benchmark"}</Button>
          </div></div>
          <p className="small dim mb">Measures CPU (single + multi thread), memory bandwidth and disk speed for ~5 seconds. It is not an in-game FPS test and only runs when you click.</p>
          {benchRunning && <div className="row"><Spinner /> Measuring…</div>}
          {bench && !benchRunning && (
            <>
              <div className="kv">
                <span>CPU single-thread</span><span>{bench.cpuSingle.toFixed(0)} units/s</span>
                <span>CPU all threads</span><span>{bench.cpuMulti.toFixed(0)} units/s ({bench.threads} threads)</span>
                <span>Memory bandwidth</span><span>{bench.memoryGbps.toFixed(1)} GB/s</span>
                <span>Disk write / read</span><span>{bench.diskWriteMbps ? `${bench.diskWriteMbps.toFixed(0)} MB/s` : "—"} / {bench.diskReadMbps ? `${bench.diskReadMbps.toFixed(0)} MB/s` : "—"}</span>
                <span>Measured</span><span>{relativeTime(bench.at)} in {(bench.durationMs / 1000).toFixed(1)} s</span>
              </div>
              <div className="mt"><Alert tone="success" title={`Recommended: ${presetLabel(bench.recommendedPreset)}`}>{bench.reasons.join("\n")}</Alert></div>
            </>
          )}
          {!bench && !benchRunning && <p className="dim">No benchmark yet.</p>}
        </div>
      </div>

      {running && (
        <div className="card mt">
          <h3>Live game process</h3>
          <div className="row wrap mt" style={{ gap: 32 }}>
            <div className="stat"><span className="v">{sample ? `${sample.memoryMb} MB` : "—"}</span><span className="l">RAM used by Java (measured)</span></div>
            <div className="stat"><span className="v">{sample ? `${sample.cpuPercent.toFixed(0)}%` : "—"}</span><span className="l">CPU (measured)</span></div>
          </div>
          <p className="small faint mt">Sampled every 2 s only while the game runs. In-game FPS and frame times are visible with F3; the launcher cannot read them and does not estimate them.</p>
        </div>
      )}

      {selected && (
        <div className="card mt">
          <div className="card-head"><h3>Graphics presets for “{selected.name}”</h3><span className="faint small">Current: {presetLabel(selected.preset)}</span></div>
          <div className="grid cols-2">
            {presets.map((p) => (
              <div key={p.id} className="choice" style={{ cursor: "default" }}>
                <div className="grow">
                  <div className="row"><strong>{p.name}</strong>{selected.preset === p.id && <Badge tone="accent">Active</Badge>}{bench?.recommendedPreset === p.id && <Badge tone="info">Recommended</Badge>}</div>
                  <div className="small dim mt">{p.description}</div>
                  <div className="row mt"><Button size="sm" onClick={async () => { const d = await run(() => api.presetPreview(selected.id, p.id)); if (d) setPreview({ id: p.id, data: d }); }}>Preview &amp; apply</Button></div>
                </div>
              </div>
            ))}
          </div>
          {current && (
            <details className="mt">
              <summary className="small dim" style={{ cursor: "pointer" }}>Current options.txt video settings</summary>
              <table className="table mt"><tbody>{Object.entries(current).map(([k, v]) => <tr key={k}><td>{k}</td><td className="mono">{v}</td></tr>)}</tbody></table>
            </details>
          )}
          {!current && <p className="small faint mt">options.txt does not exist yet; the selected preset is written safely the first time this profile starts.</p>}
        </div>
      )}

      {custom && settings && (
        <div className="card mt">
          <div className="card-head"><h3>Custom preset</h3><div className="actions"><Button size="sm" variant="primary" disabled={customWarn.some((w) => w.includes("must be"))} onClick={() => run(() => saveSettings({ ...settings, customGraphics: custom }), "Custom preset saved")}>Save custom preset</Button></div></div>
          <div className="form-grid">
            <Slider label="Render distance" value={custom.renderDistance} min={2} max={32} onChange={(v) => setCustom({ ...custom, renderDistance: v })} suffix=" chunks" />
            <Slider label="Simulation distance" value={custom.simulationDistance} min={5} max={32} onChange={(v) => setCustom({ ...custom, simulationDistance: v })} suffix=" chunks" />
            <Slider label="FPS limit" value={custom.maxFps} min={10} max={260} step={10} onChange={(v) => setCustom({ ...custom, maxFps: v })} format={(v) => (v >= 260 ? "Unlimited" : `${v} FPS`)} />
            <Slider label="Entity distance" value={custom.entityDistance} min={50} max={500} step={25} onChange={(v) => setCustom({ ...custom, entityDistance: v })} suffix="%" />
            <Slider label="Mipmap levels" value={custom.mipmapLevels} min={0} max={4} onChange={(v) => setCustom({ ...custom, mipmapLevels: v })} />
            <Slider label="Biome blend" value={custom.biomeBlend} min={0} max={7} onChange={(v) => setCustom({ ...custom, biomeBlend: v })} />
            <Field label="Particles"><Select value={String(custom.particles)} onChange={(v) => setCustom({ ...custom, particles: Number(v) })} options={[{ value: "0", label: "All" }, { value: "1", label: "Decreased" }, { value: "2", label: "Minimal" }]} /></Field>
            <Field label="Graphics"><Select value={custom.graphics} onChange={(v) => setCustom({ ...custom, graphics: v })} options={[{ value: "fast", label: "Fast" }, { value: "fancy", label: "Fancy" }]} /></Field>
            <Field label="Clouds"><Select value={custom.clouds} onChange={(v) => setCustom({ ...custom, clouds: v })} options={[{ value: "off", label: "Off" }, { value: "fast", label: "Fast" }, { value: "fancy", label: "Fancy" }]} /></Field>
            <div className="col">
              <label className="row"><Toggle on={custom.vsync} onChange={(v) => setCustom({ ...custom, vsync: v })} label="VSync" /> VSync</label>
              <label className="row"><Toggle on={custom.entityShadows} onChange={(v) => setCustom({ ...custom, entityShadows: v })} label="Entity shadows" /> Entity shadows</label>
              <label className="row"><Toggle on={custom.smoothLighting} onChange={(v) => setCustom({ ...custom, smoothLighting: v })} label="Smooth lighting" /> Smooth lighting</label>
            </div>
          </div>
          {customWarn.map((w, i) => <div className="mt" key={i}><Alert tone="warning">{w}</Alert></div>)}
        </div>
      )}

      {selected && (
        <div className="grid cols-2 mt">
          <div className="card">
            <h3>Settings backups</h3>
            <p className="small dim mt mb">A copy of options.txt is saved before every change. “Original” is the file as it was before Shadow Client first changed it — restoring it is “restore defaults”.</p>
            {backups.length === 0 && <p className="dim">No backups yet.</p>}
            {backups.slice(0, 8).map((b) => (
              <div className="row mb" key={b.name}>
                <span className="grow small">{b.original ? <Badge tone="accent">Original</Badge> : null} {b.name}</span>
                <Button size="sm" icon={<RotateCcw size={14} />} onClick={() => run(async () => { await api.optionsRestore(selected.id, b.name); await loadProfileBits(); }, "Settings restored")}>Restore</Button>
              </div>
            ))}
          </div>
          <div className="card">
            <h3>Memory</h3>
            {mem && (
              <>
                <div className="kv mt">
                  <span>Recommended</span><span>{(mem.maxMb / 1024).toFixed(1)} GB max · {(mem.minMb / 1024).toFixed(1)} GB min</span>
                  <span>Upper limit on this PC</span><span>{(mem.ceilingMb / 1024).toFixed(1)} GB</span>
                </div>
                <p className="small dim mt">{mem.reasons.join(". ")}. More memory does not mean more FPS: an oversized heap makes garbage-collection pauses longer. Change it per profile (Profiles → Edit) or globally (Settings → Minecraft).</p>
              </>
            )}
          </div>
        </div>
      )}

      {preview && selected && (
        <Modal wide title={`Apply ${presetLabel(preview.id)} to ${selected.name}`} onClose={() => setPreview(null)} footer={<>
          <Button onClick={() => setPreview(null)}>Cancel</Button>
          <Button variant="primary" disabled={running} onClick={async () => { const r = await run(() => api.applyPreset(selected.id, preview.id)); if (r) { setApplied(r); setPreview(null); await refreshProfiles(); await loadProfileBits(); } }}>{preview.data.optionsExists ? `Apply ${preview.data.changes.length} change(s)` : "Save as profile preset"}</Button>
        </>}>
          {running && <Alert tone="warning">Close Minecraft first: it rewrites options.txt when it exits.</Alert>}
          {!preview.data.optionsExists && <Alert tone="info">options.txt doesn't exist yet. The preset is stored on the profile and written with the correct data version on first start.</Alert>}
          {preview.data.optionsExists && preview.data.changes.length === 0 && <Alert tone="success">options.txt already matches this preset.</Alert>}
          {preview.data.changes.length > 0 && (
            <table className="table mt"><thead><tr><th>Setting</th><th>Now</th><th>After</th></tr></thead>
              <tbody>{preview.data.changes.map(([k, o, n]) => <tr key={k}><td>{k}</td><td className="mono">{o || "—"}</td><td className="mono">{n}</td></tr>)}</tbody>
            </table>
          )}
          {preview.data.warnings.map((w, i) => <div className="mt" key={i}><Alert tone="warning">{w}</Alert></div>)}
          <p className="small faint mt">Only these known keys are changed; every other line (keybinds, mod settings, resource packs) is preserved. A backup is taken first.</p>
        </Modal>
      )}
      {applied && (
        <Modal title="Preset applied" onClose={() => setApplied(null)} footer={<Button variant="primary" onClick={() => setApplied(null)}>Done</Button>}>
          <Alert tone="success" title={applied.created ? "options.txt created" : `${applied.changed.length} setting(s) changed`}>{applied.backup ? `Backup saved: ${applied.backup}` : "No previous file to back up."}</Alert>
        </Modal>
      )}
    </div>
  );
}

function Slider({ label, value, min, max, step = 1, onChange, suffix = "", format }: { label: string; value: number; min: number; max: number; step?: number; onChange: (v: number) => void; suffix?: string; format?: (v: number) => string }) {
  return (
    <Field label={`${label}: ${format ? format(value) : `${value}${suffix}`}`}>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} />
    </Field>
  );
}
