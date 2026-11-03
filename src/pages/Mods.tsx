import { useCallback, useEffect, useMemo, useState } from "react";
import { ArrowUpCircle, ExternalLink, FolderOpen, Info, RefreshCw, RotateCcw, Search, Trash2 } from "lucide-react";
import { api, type BackupEntry, type InstalledMod, type PerfModStatus, type PlanResult } from "../api";
import { NewProfileDialog } from "../components/NewProfileDialog";
import { PlanDialog } from "../components/PlanDialog";
import { ProjectBrowser } from "../components/ProjectBrowser";
import { Alert, Badge, Button, Confirm, Empty, Img, Modal, Spinner, Tabs, Toggle } from "../components/ui";
import { bytes, loaderLabel, relativeTime } from "../format";
import { useStore } from "../store";
import { ResourcePacks, Shaders } from "./Packs";

type Tab = "installed" | "updates" | "modrinth" | "curseforge" | "performance" | "resourcepacks" | "shaders" | "recent" | "backups";

export function Mods() {
  const { selected, subPage } = useStore();
  // "installed|sodium" opens a tab with its search filled in (from Ctrl+K)
  const [subTab, subQuery] = (subPage ?? "").split("|");
  const [tab, setTab] = useState<Tab>((subTab as Tab) || "installed");
  const [createFor, setCreateFor] = useState<string | null>(null);
  if (!selected) return <div className="page"><Empty title="Select or create a profile first" /></div>;
  const noMods = selected.loader === "vanilla";
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Mods</h1>
          <div className="sub">{selected.name} · Minecraft {selected.mcVersion} · {loaderLabel(selected.loader)}</div>
        </div>
      </div>
      {noMods && <div className="mb"><Alert tone="info" title="Vanilla profile">Vanilla Minecraft can't load mods. Create a Fabric profile (Profiles → New) to use mods; this profile stays unchanged.</Alert></div>}
      <Tabs<Tab>
        value={tab}
        onChange={setTab}
        tabs={[
          { id: "installed", label: "Installed" },
          { id: "updates", label: "Updates" },
          { id: "modrinth", label: "Modrinth" },
          { id: "curseforge", label: "CurseForge" },
          { id: "performance", label: "Performance mods" },
          { id: "resourcepacks", label: "Resource packs" },
          { id: "shaders", label: "Shaders" },
          { id: "recent", label: "Recently viewed" },
          { id: "backups", label: "Backups" },
        ]}
      />
      {tab === "installed" && <Installed initialFilter={subTab === "installed" ? subQuery : undefined} />}
      {tab === "updates" && <Updates />}
      {tab === "modrinth" && <ProjectBrowser platform="modrinth" kind="mod" initialQuery={subTab === "modrinth" ? subQuery : undefined} onCreateProfile={setCreateFor} />}
      {tab === "curseforge" && <ProjectBrowser platform="curseforge" kind="mod" onCreateProfile={setCreateFor} />}
      {tab === "performance" && <PerformanceMods />}
      {tab === "resourcepacks" && <ResourcePacks />}
      {tab === "shaders" && <Shaders />}
      {tab === "recent" && <Recent />}
      {tab === "backups" && <Backups />}
      {createFor && <NewProfileDialog initialVersion={createFor} initialKind="modded" onClose={() => setCreateFor(null)} />}
    </div>
  );
}

function useMods(identify: boolean) {
  const { selected } = useStore();
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    if (!selected) return;
    setError(null);
    try {
      setMods((await api.modsList(selected.id, identify)).filter((m) => !m.builtin)); // built-in client mods aren't listed
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [selected, identify]);
  useEffect(() => {
    load();
  }, [load]);
  return { mods, setMods, error, load };
}

function sourceLabel(m: InstalledMod): string {
  const s = m.source?.source;
  if (s === "modrinth") return "Modrinth";
  if (s === "curseforge") return "CurseForge";
  if (s === "modpack") return "Modpack";
  if (m.identified) return "Modrinth (identified)";
  return "Manual";
}

function Installed({ initialFilter }: { initialFilter?: string }) {
  const { selected, run, games } = useStore();
  const { mods, error, load } = useMods(true);
  const [filter, setFilter] = useState(initialFilter ?? "");
  const [remove, setRemove] = useState<InstalledMod | null>(null);
  const [details, setDetails] = useState<InstalledMod | null>(null);
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const running = selected ? games[selected.id]?.state === "running" : false;
  const shown = useMemo(() => (mods ?? []).filter((m) => `${m.meta?.name ?? ""} ${m.source?.title ?? ""} ${m.identified?.title ?? ""} ${m.fileName}`.toLowerCase().includes(filter.toLowerCase())), [mods, filter]);
  if (!selected) return null;
  const warnings = (mods ?? []).filter((m) => m.enabled && m.warnings.length > 0).length;
  return (
    <div>
      <div className="row wrap mb">
        <Search size={16} className="faint" />
        <input className="input grow" placeholder="Filter installed mods…" value={filter} onChange={(e) => setFilter(e.target.value)} />
        <Button size="sm" icon={<RefreshCw size={14} />} onClick={load}>Refresh</Button>
        <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openProfileFolder(selected.id, "mods"))}>Open folder</Button>
      </div>
      {running && <div className="mb"><Alert tone="info">Minecraft is running. Changes take effect after you restart the game.</Alert></div>}
      {warnings > 0 && <div className="mb"><Alert tone="warning" title={`${warnings} mod(s) have warnings`}>Use the ⓘ button on a mod for details. Missing dependencies and conflicts usually stop the game from starting.</Alert></div>}
      {error && <Alert tone="error">{error}</Alert>}
      {!mods && !error && <div className="row dim"><Spinner /> Reading mods…</div>}
      {mods && mods.length === 0 && <Empty title="No mods installed">Browse Modrinth or CurseForge, or install the recommended performance mods.</Empty>}
      {shown.length > 0 && (
        <div className="list">
          {shown.map((m) => (
            <div key={m.fileName} className={`list-row ${m.enabled ? "" : "disabled"}`}>
              <Img src={m.icon ?? m.source?.iconUrl ?? m.identified?.iconUrl} />
              <div className="grow" style={{ minWidth: 0 }}>
                <div className="row">
                  <strong className="ellipsis">{m.meta?.name ?? m.source?.title ?? m.identified?.title ?? m.fileName}</strong>
                  <span className="small faint">{m.meta?.version ?? m.source?.versionNumber ?? ""}</span>
                </div>
                <div className="small faint ellipsis">{m.fileName} · {bytes(m.size)} · {sourceLabel(m)}{m.meta ? ` · ${loaderLabel(m.meta.loader)}` : ""}</div>
                {m.enabled && m.warnings.length > 0 && <div className="small" style={{ color: "var(--warning)" }}>{m.warnings[0]}{m.warnings.length > 1 ? ` (+${m.warnings.length - 1})` : ""}</div>}
              </div>
              {m.update && <Button size="sm" icon={<ArrowUpCircle size={14} />} onClick={async () => { const r = await run(() => api.planUpdate(selected.id, m.fileName)); if (r) setPlan(r); }}>Update to {m.update.versionNumber}</Button>}
              <Button size="sm" variant="ghost" icon={<Info size={14} />} title="Details" onClick={() => setDetails(m)} />
              <Toggle on={m.enabled} label="Enabled" onChange={(v) => run(async () => { await api.modSetEnabled(selected.id, m.fileName, v); await load(); })} />
              <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={() => setRemove(m)} />
            </div>
          ))}
        </div>
      )}
      {remove && (
        <Confirm title={`Remove ${remove.meta?.name ?? remove.fileName}?`} confirmLabel="Remove" danger onCancel={() => setRemove(null)} onConfirm={async () => {
          const r = await run(() => api.modRemove(selected.id, remove.fileName));
          setRemove(null);
          await load();
          if (r && r.dependents.length) await run(async () => { throw new Error(`Removed. These mods depended on it and may stop working: ${r.dependents.join(", ")}`); });
        }}>
          <p>The file is moved to this profile's backups (Mods → Backups), not deleted, so you can restore it.</p>
        </Confirm>
      )}
      {details && <ModDetails mod={details} onClose={() => setDetails(null)} />}
      {plan && <PlanDialog result={plan} onClose={() => setPlan(null)} onDone={() => load()} />}
    </div>
  );
}

function ModDetails({ mod, onClose }: { mod: InstalledMod; onClose: () => void }) {
  const { run } = useStore();
  const url = mod.source?.websiteUrl ?? (mod.identified ? `https://modrinth.com/mod/${mod.identified.slug ?? mod.identified.projectId}` : mod.meta?.homepage);
  return (
    <Modal title={mod.meta?.name ?? mod.fileName} onClose={onClose} footer={url?.startsWith("https://") ? <Button icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(url))}>Open project page</Button> : undefined}>
      {mod.meta?.description && <p className="mb">{mod.meta.description}</p>}
      <div className="kv">
        <span>File</span><span>{mod.fileName}</span>
        <span>Mod id</span><span>{mod.meta?.modId ?? "unknown"}</span>
        <span>Version</span><span>{mod.meta?.version ?? "—"}</span>
        <span>Loader</span><span>{mod.meta ? loaderLabel(mod.meta.loader) : "—"}</span>
        <span>Authors</span><span>{mod.meta?.authors.join(", ") || "—"}</span>
        <span>License</span><span>{mod.meta?.license ?? "Not stated"}</span>
        <span>Source</span><span>{sourceLabel(mod)}</span>
        <span>SHA-1</span><span className="mono">{mod.sha1}</span>
        {mod.source?.installedAt && <><span>Installed</span><span>{relativeTime(mod.source.installedAt)}</span></>}
        {mod.source?.requiredBy && mod.source.requiredBy.length > 0 && <><span>Required by</span><span>{mod.source.requiredBy.join(", ")}</span></>}
      </div>
      {mod.meta && Object.keys(mod.meta.depends).length > 0 && (
        <>
          <h3 className="mt mb">Dependencies</h3>
          <table className="table"><tbody>{Object.entries(mod.meta.depends).map(([k, v]) => <tr key={k}><td>{k}</td><td className="mono">{v.join(" or ")}</td></tr>)}</tbody></table>
        </>
      )}
      {mod.meta && Object.keys(mod.meta.breaks).length > 0 && (
        <>
          <h3 className="mt mb">Declared incompatibilities</h3>
          <table className="table"><tbody>{Object.entries(mod.meta.breaks).map(([k, v]) => <tr key={k}><td>{k}</td><td className="mono">{v.join(" or ")}</td></tr>)}</tbody></table>
        </>
      )}
      {mod.warnings.map((w, i) => <div className="mt" key={i}><Alert tone="warning">{w}</Alert></div>)}
    </Modal>
  );
}

function Updates() {
  const { selected, run } = useStore();
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [checking, setChecking] = useState(false);
  const [plan, setPlan] = useState<PlanResult | null>(null);
  if (!selected) return null;
  const check = async () => {
    setChecking(true);
    const r = await run(() => api.modsCheckUpdates(selected.id));
    setChecking(false);
    if (r) setMods(r.filter((m) => !m.builtin));
  };
  const withUpdates = (mods ?? []).filter((m) => m.update);
  return (
    <div>
      <div className="row mb">
        <p className="dim grow">Updates are only offered for the same Minecraft version and loader; a mod never moves to a build for another version.</p>
        <Button variant="primary" icon={<RefreshCw size={14} />} onClick={check} disabled={checking}>{checking ? "Checking…" : "Check for updates"}</Button>
      </div>
      {mods && withUpdates.length === 0 && <Alert tone="success" title="Everything is up to date">{mods.length} mods checked. Mods that Modrinth doesn't know (manual files) can't be checked.</Alert>}
      {withUpdates.length > 0 && (
        <div className="list">
          {withUpdates.map((m) => (
            <div className="list-row" key={m.fileName}>
              <Img src={m.icon ?? m.identified?.iconUrl} />
              <div className="grow"><strong>{m.meta?.name ?? m.fileName}</strong><div className="small faint">{m.meta?.version} → {m.update!.versionNumber}</div></div>
              <Button size="sm" variant="primary" onClick={async () => { const r = await run(() => api.planUpdate(selected.id, m.fileName)); if (r) setPlan(r); }}>Update</Button>
            </div>
          ))}
        </div>
      )}
      {plan && <PlanDialog result={plan} onClose={() => setPlan(null)} onDone={() => check()} />}
    </div>
  );
}

function PerformanceMods() {
  const { selected, run } = useStore();
  const [list, setList] = useState<PerfModStatus[] | null>(null);
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const load = useCallback(async () => {
    if (!selected) return;
    try {
      setList(await api.performanceModsStatus(selected.id));
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e));
    }
  }, [selected]);
  useEffect(() => { load(); }, [load]);
  if (!selected) return null;
  const recommended = (list ?? []).filter((m) => m.recommended && m.available && !m.installedFile);
  return (
    <div>
      <div className="row mb">
        <p className="dim grow">Availability is checked live on Modrinth for Minecraft {selected.mcVersion} on {loaderLabel(selected.loader)}. Only one renderer mod can be installed; conflicts are blocked automatically.</p>
        <Button variant="primary" disabled={recommended.length === 0} onClick={async () => { const r = await run(() => api.installPerformanceMods(selected.id, recommended.map((m) => m.slug))); if (r) setPlan(r); }}>Install recommended ({recommended.length})</Button>
      </div>
      {err && <Alert tone="error">{err}</Alert>}
      {!list && !err && <div className="row dim"><Spinner /> Checking Modrinth…</div>}
      {list && (
        <div className="list">
          {list.map((m) => (
            <div className="list-row" key={m.slug}>
              <div className="grow">
                <div className="row"><strong>{m.name}</strong><Badge>{m.role}</Badge>{m.recommended && <Badge tone="accent">Recommended for {selected.preset ?? "balanced"}</Badge>}</div>
                <div className="small dim">{m.summary}</div>
                <div className="small faint">{m.available === true ? `Compatible build: ${m.availableVersion}` : m.note}</div>
              </div>
              {m.installedFile ? (
                <>
                  <Badge tone="accent">Installed</Badge>
                  <Toggle on={m.enabled} label="Enabled" onChange={(v) => run(async () => { await api.modSetEnabled(selected.id, m.installedFile!, v); await load(); })} />
                </>
              ) : (
                <Button size="sm" disabled={m.available !== true} onClick={async () => { const r = await run(() => api.planInstall({ profileId: selected.id, platform: "modrinth", projectId: m.projectId, versionId: null, kind: "mod" })); if (r) setPlan(r); }}>Install</Button>
              )}
            </div>
          ))}
        </div>
      )}
      {plan && <PlanDialog result={plan} onClose={() => setPlan(null)} onDone={() => load()} />}
    </div>
  );
}

function Recent() {
  const { appState, run } = useStore();
  const list = appState?.recentlyViewed ?? [];
  if (list.length === 0) return <Empty title="Nothing viewed yet">Projects you open on Modrinth or CurseForge appear here.</Empty>;
  return (
    <div className="list">
      {list.map((r) => (
        <div className="list-row" key={r.platform + r.id}>
          <Img src={r.iconUrl} />
          <div className="grow"><strong>{r.title}</strong><div className="small faint">{r.platform === "modrinth" ? "Modrinth" : "CurseForge"} · {r.projectType ?? "project"} · viewed {relativeTime(r.viewedAt)}</div></div>
          <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(r.platform === "modrinth" ? `https://modrinth.com/${r.projectType ?? "mod"}/${r.slug ?? r.id}` : `https://www.curseforge.com/minecraft/mc-mods/${r.slug ?? r.id}`))}>Open page</Button>
        </div>
      ))}
    </div>
  );
}

function Backups() {
  const { selected, run } = useStore();
  const [list, setList] = useState<BackupEntry[] | null>(null);
  const load = useCallback(async () => { if (selected) setList(await api.modBackups(selected.id)); }, [selected]);
  useEffect(() => { load(); }, [load]);
  if (!selected) return null;
  if (list && list.length === 0) return <Empty title="No backups">Removed, replaced and updated mods are kept here so you can restore a previous version.</Empty>;
  return (
    <div className="list">
      {list?.map((b) => (
        <div className="list-row" key={b.path}>
          <div className="grow"><strong>{b.title ?? b.fileName}</strong><div className="small faint">{b.fileName} · saved {b.takenAt}</div></div>
          <Button size="sm" icon={<RotateCcw size={14} />} onClick={() => run(async () => { await api.modRestore(selected.id, b.path); await load(); }, "Restored")}>Restore</Button>
        </div>
      ))}
    </div>
  );
}
