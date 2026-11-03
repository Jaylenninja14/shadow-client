import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Download, ExternalLink, FileUp, FolderOpen, Play, RefreshCw, Search, Trash2 } from "lucide-react";
import { api, type BedrockImportReport, type BedrockPack, type BedrockView, type CfMod } from "../api";
import { Alert, Badge, Button, Confirm, Empty, Img, Modal, Spinner, Tabs } from "../components/ui";
import { useStore } from "../store";
import { BedrockModMenu } from "../components/BedrockModMenu";
import { BedrockWorlds } from "../components/BedrockWorlds";

// Minecraft Bedrock (Windows): start it, change it through its mod menu (Bedrock's own settings
// plus Shadow Client's resource pack), and manage resource/behavior packs, including ones from
// CurseForge. Bedrock itself is a Microsoft Store app; Windows starts it.

type Tab = "menu" | "worlds" | "installed" | "curseforge";

export function Bedrock() {
  const { run, subPage } = useStore();
  const [info, setInfo] = useState<BedrockView | null>(null);
  // "menu|Max Framerate" opens the mod menu searching for that module (from Ctrl+K)
  const [subTab, subQuery] = (subPage ?? "").split("|");
  const [tab, setTab] = useState<Tab>((subTab as Tab) || "menu");
  const [report, setReport] = useState<BedrockImportReport | null>(null);
  const [packsVersion, setPacksVersion] = useState(0);

  useEffect(() => { api.bedrockInfo().then(setInfo).catch(() => setInfo(null)); }, []);
  const refresh = () => setPacksVersion((v) => v + 1);

  const importFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Bedrock packs", extensions: ["mcpack", "mcaddon", "zip", "mcworld", "mctemplate"] }] });
    if (typeof path !== "string") return;
    const r = await run(() => api.bedrockImport(path));
    if (r) {
      setReport(r);
      refresh();
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Bedrock</h1>
          <div className="sub">Minecraft Bedrock Edition for Windows{info?.version ? ` · ${info.version}` : ""}</div>
        </div>
        <div className="actions">
          {info?.dataDir && <Button icon={<FolderOpen size={14} />} onClick={() => run(() => api.bedrockOpenFolder(null))}>Open folder</Button>}
          {info?.dataDir && <Button icon={<FileUp size={14} />} onClick={importFile}>Import file…</Button>}
        </div>
      </div>

      {!info && <Spinner />}
      {info && (
        <div className="hero art">
          <div className="row wrap" style={{ alignItems: "center", minHeight: 170 }}>
            <div className="grow col" style={{ gap: 4, minWidth: 260 }}>
              <div className="hero-title">Minecraft Bedrock</div>
              {info.installed ? (
                <p className="dim">Installed{info.version ? ` (version ${info.version})` : ""}. Packs you add here show up in Bedrock: resource packs under Settings → Global Resources, behavior packs in a world's settings.</p>
              ) : (
                <p className="dim">Bedrock isn't installed on this PC. It's sold separately from Java Edition (owners of Java Edition on PC get it too) and comes from the Microsoft Store.</p>
              )}
            </div>
            {info.installed ? (
              <Button className="play-btn" variant="primary" icon={<Play size={18} />} onClick={() => run(() => api.bedrockLaunch())}>PLAY BEDROCK</Button>
            ) : (
              <Button variant="primary" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(info.storeUrl))}>Get it from the Microsoft Store</Button>
            )}
          </div>
        </div>
      )}

      {info?.dataDir && (
        <>
          <div className="mt">
            <Tabs<Tab> value={tab} onChange={setTab} tabs={[{ id: "menu", label: "Mod menu" }, { id: "worlds", label: "Worlds" }, { id: "installed", label: "Installed packs" }, { id: "curseforge", label: "CurseForge" }]} />
          </div>
          {tab === "menu" && <BedrockModMenu initialQuery={subTab === "menu" ? subQuery : undefined} />}
          {tab === "worlds" && <BedrockWorlds />}
          {tab === "installed" && <Installed key={packsVersion} />}
          {tab === "curseforge" && <CurseForge hasKey={info.curseforgeKey} onInstalled={(r) => { setReport(r); refresh(); }} />}
        </>
      )}
      {info && info.installed && !info.dataDir && <div className="mt"><Alert tone="info">Start Bedrock once so it creates its data folder; then packs can be managed here.</Alert></div>}

      {report && <ReportModal report={report} onClose={() => setReport(null)} />}
    </div>
  );
}

const KIND_LABEL: Record<string, string> = { resource: "Resource pack", behavior: "Behavior pack", skin: "Skin pack" };

function Installed() {
  const { run } = useStore();
  const [packs, setPacks] = useState<BedrockPack[] | null>(null);
  const [filter, setFilter] = useState("");
  const [kind, setKind] = useState<"all" | "resource" | "behavior" | "skin">("all");
  const [remove, setRemove] = useState<BedrockPack | null>(null);
  const load = useCallback(async () => setPacks(await api.bedrockPacks()), []);
  useEffect(() => { load().catch(() => setPacks([])); }, [load]);
  const shown = (packs ?? []).filter((p) => (kind === "all" || p.kind === kind) && `${p.name} ${p.description}`.toLowerCase().includes(filter.toLowerCase()));
  return (
    <div>
      <div className="row wrap mb">
        <Search size={16} className="faint" />
        <input className="input grow" placeholder="Filter packs…" value={filter} onChange={(e) => setFilter(e.target.value)} />
        <select className="select" style={{ width: 150 }} value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
          <option value="all">All packs</option>
          <option value="resource">Resource packs</option>
          <option value="behavior">Behavior packs</option>
          <option value="skin">Skin packs</option>
        </select>
        <Button size="sm" icon={<RefreshCw size={14} />} onClick={load}>Refresh</Button>
      </div>
      {!packs && <Spinner />}
      {packs && packs.length === 0 && <Empty title="No packs yet">Import a .mcpack / .mcaddon file or install one from CurseForge.</Empty>}
      {shown.length > 0 && (
        <div className="list">
          {shown.map((p) => (
            <div key={`${p.kind}/${p.development}/${p.folder}`} className="list-row">
              <div className="grow" style={{ minWidth: 0 }}>
                <div className="row">
                  <strong className="ellipsis">{p.name || p.folder}</strong>
                  <span className="small faint">{p.version}</span>
                  <Badge>{KIND_LABEL[p.kind]}</Badge>
                  {p.development && <Badge tone="info">Development</Badge>}
                </div>
                {p.description && <div className="small dim ellipsis">{p.description}</div>}
              </div>
              <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={() => setRemove(p)} />
            </div>
          ))}
        </div>
      )}
      <p className="small faint mt">Packs bought on the Minecraft Marketplace are managed by Bedrock itself and aren't listed here.</p>
      {remove && (
        <Confirm title={`Remove ${remove.name || remove.folder}?`} confirmLabel="Remove" danger onCancel={() => setRemove(null)} onConfirm={async () => {
          await run(() => api.bedrockRemove(remove.kind, remove.folder, remove.development), "Pack moved to Shadow Client's backups");
          setRemove(null);
          await load();
        }}>
          The pack folder is moved to Shadow Client's backups, not deleted. Worlds that use it will ask for it the next time they load.
        </Confirm>
      )}
    </div>
  );
}

function CurseForge({ hasKey, onInstalled }: { hasKey: boolean; onInstalled: (r: BedrockImportReport) => void }) {
  const { run, setPage } = useStore();
  const [classes, setClasses] = useState<{ id: number; name: string }[] | null>(null);
  const [classId, setClassId] = useState<number | null>(null);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState("popularity");
  const [results, setResults] = useState<CfMod[] | null>(null);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<number | null>(null);

  useEffect(() => {
    if (!hasKey) return;
    api.bedrockCfClasses().then((c) => {
      const usable = c.filter((x) => /add|pack|texture|resource|behav|script/i.test(x.name));
      const list = usable.length ? usable : c;
      setClasses(list);
      setClassId(list[0]?.id ?? null);
    }).catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [hasKey]);

  useEffect(() => {
    if (classId == null) return;
    setError(null);
    const t = setTimeout(() => {
      api.bedrockCfSearch(classId, query, sort, offset).then((r) => { setResults(r.data); setTotal(r.pagination.totalCount); }).catch((e) => setError(e instanceof Error ? e.message : String(e)));
    }, 300);
    return () => clearTimeout(t);
  }, [classId, query, sort, offset]);

  if (!hasKey) {
    return (
      <Alert tone="info" title="CurseForge needs an API key" action={<Button size="sm" onClick={() => setPage("settings")}>Open Settings</Button>}>
        CurseForge only allows its downloads through apps with an API key. Create a free key at console.curseforge.com and add it under Settings → Accounts &amp; services (docs/CURSEFORGE_SETUP.md). Until then, download packs on curseforge.com/minecraft-bedrock and use Import file.
      </Alert>
    );
  }

  const install = async (m: CfMod) => {
    setBusy(m.id);
    const r = await run(() => api.bedrockCfInstall(m.id));
    setBusy(null);
    if (r) onInstalled(r);
  };

  return (
    <div>
      <div className="row wrap mb">
        {classes?.map((c) => (
          <Button key={c.id} size="sm" variant={c.id === classId ? "primary" : "default"} onClick={() => { setClassId(c.id); setOffset(0); }}>{c.name}</Button>
        ))}
      </div>
      <div className="row wrap mb">
        <Search size={16} className="faint" />
        <input className="input grow" placeholder="Search Bedrock packs on CurseForge…" value={query} onChange={(e) => { setQuery(e.target.value); setOffset(0); }} />
        <select className="select" style={{ width: 150 }} value={sort} onChange={(e) => { setSort(e.target.value); setOffset(0); }}>
          <option value="popularity">Popular</option>
          <option value="downloads">Most downloaded</option>
          <option value="updated">Recently updated</option>
          <option value="name">Name</option>
        </select>
      </div>
      {error && <Alert tone="error">{error}</Alert>}
      {!results && !error && <Spinner />}
      {results && results.length === 0 && <Empty title="Nothing found" />}
      {results && results.length > 0 && (
        <div className="list">
          {results.map((m) => (
            <div key={m.id} className="list-row">
              <Img src={m.logo?.thumbnailUrl} />
              <div className="grow" style={{ minWidth: 0 }}>
                <div className="row"><strong className="ellipsis">{m.name}</strong><span className="small faint">by {m.authors.map((a) => a.name).join(", ")}</span></div>
                <div className="small dim ellipsis">{m.summary}</div>
              </div>
              {m.links?.websiteUrl && <Button size="sm" variant="ghost" icon={<ExternalLink size={14} />} title="CurseForge page" onClick={() => run(() => api.openUrl(m.links!.websiteUrl!))} />}
              {m.allowModDistribution === false ? (
                <span className="small faint" title="The author only allows downloads on CurseForge's website">Website only</span>
              ) : (
                <Button size="sm" variant="primary" icon={<Download size={14} />} disabled={busy !== null} onClick={() => install(m)}>{busy === m.id ? "Installing…" : "Install"}</Button>
              )}
            </div>
          ))}
        </div>
      )}
      {results && total > offset + 24 && (
        <div className="row mt">
          {offset > 0 && <Button size="sm" onClick={() => setOffset(Math.max(0, offset - 24))}>Previous</Button>}
          <span className="grow" />
          <Button size="sm" onClick={() => setOffset(offset + 24)}>Next</Button>
        </div>
      )}
      <p className="small faint mt">Downloads come from CurseForge's CDN over HTTPS and are checked against CurseForge's published hash before the pack is unpacked.</p>
    </div>
  );
}

function ReportModal({ report, onClose }: { report: BedrockImportReport; onClose: () => void }) {
  return (
    <Modal title={report.openedInBedrock ? "Opened in Bedrock" : "Pack installed"} onClose={onClose} footer={<Button variant="primary" onClick={onClose}>Done</Button>}>
      {report.openedInBedrock && <p>Worlds and world templates are imported by Bedrock itself. It opens and shows the import progress.</p>}
      {report.installed.length > 0 && <Alert tone="success" title="Installed">{report.installed.join("\n")}</Alert>}
      {report.replaced.length > 0 && <div className="mt"><Alert tone="info" title="Updated (old version kept in backups)">{report.replaced.join("\n")}</Alert></div>}
      {report.skipped.length > 0 && <div className="mt"><Alert tone="warning" title="Skipped">{report.skipped.join("\n")}</Alert></div>}
      {!report.openedInBedrock && <p className="small dim mt">Turn resource packs on in Bedrock under Settings → Global Resources; add behavior packs in a world's settings (they need a world with experiments allowed if the pack says so).</p>}
    </Modal>
  );
}
