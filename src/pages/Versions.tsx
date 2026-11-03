import { useEffect, useState } from "react";
import { Coffee, Download, ExternalLink, RefreshCw, Wrench } from "lucide-react";
import { api, type JavaInstall, type VersionList } from "../api";
import { NewProfileDialog } from "../components/NewProfileDialog";
import { Alert, Badge, Button, Spinner, Tabs, Toggle } from "../components/ui";
import { relativeTime } from "../format";
import { useStore } from "../store";

type Tab = "minecraft" | "loaders" | "java";

const RUNTIMES = [
  { component: "jre-legacy", label: "Java 8 (Minecraft ≤ 1.16)" },
  { component: "java-runtime-alpha", label: "Java 16 (Minecraft 1.17)" },
  { component: "java-runtime-gamma", label: "Java 17 (Minecraft 1.18 – 1.20.4)" },
  { component: "java-runtime-delta", label: "Java 21 (Minecraft 1.20.5 – 1.21.x)" },
  { component: "java-runtime-epsilon", label: "Java 25 (Minecraft 26.x)" },
];

export function Versions() {
  const { settings, selected, run } = useStore();
  const [tab, setTab] = useState<Tab>("minecraft");
  const [snapshots, setSnapshots] = useState(settings?.showSnapshots ?? false);
  const [old, setOld] = useState(settings?.showOldVersions ?? false);
  const [list, setList] = useState<VersionList | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [fabric, setFabric] = useState<Set<string> | null>(null);
  const [quilt, setQuilt] = useState<Set<string> | null>(null);
  const [javas, setJavas] = useState<JavaInstall[] | null>(null);
  const [create, setCreate] = useState<string | null>(null);

  const load = async (refresh = false) => {
    setError(null);
    try {
      setList(await api.listVersions(snapshots, old, refresh));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  useEffect(() => { load(); /* eslint-disable-next-line react-hooks/exhaustive-deps */ }, [snapshots, old]);
  useEffect(() => {
    api.loaderGameVersions("fabric").then((v) => setFabric(new Set(v.map((x) => x.version)))).catch(() => setFabric(new Set()));
    api.loaderGameVersions("quilt").then((v) => setQuilt(new Set(v.map((x) => x.version)))).catch(() => setQuilt(new Set()));
  }, []);
  useEffect(() => { if (tab === "java" && !javas) api.listJava().then(setJavas); }, [tab, javas]);

  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Versions</h1><div className="sub">Official Mojang version list, loader support and Java runtimes.</div></div>
        <div className="actions">
          {selected && <Button icon={<Wrench size={14} />} onClick={() => run(() => api.repairProfile(selected.id), "Files verified")}>Repair "{selected.name}"</Button>}
        </div>
      </div>
      <Tabs<Tab> value={tab} onChange={setTab} tabs={[{ id: "minecraft", label: "Minecraft" }, { id: "loaders", label: "Mod loaders" }, { id: "java", label: "Java" }]} />
      {tab === "minecraft" && (
        <div>
          <div className="row mb wrap">
            <label className="row small"><Toggle on={snapshots} onChange={setSnapshots} label="Snapshots" /> Snapshots</label>
            <label className="row small"><Toggle on={old} onChange={setOld} label="Old versions" /> Old alpha/beta</label>
            <span className="grow" />
            <Button size="sm" icon={<RefreshCw size={14} />} onClick={() => load(true)}>Refresh</Button>
          </div>
          {error && <Alert tone="error">{error}</Alert>}
          {!list && !error && <div className="row dim"><Spinner /> Loading Mojang's version list…</div>}
          {list && (
            <div className="list">
              {list.versions.slice(0, 150).map((v) => (
                <div className="list-row" key={v.id} style={{ minHeight: 46 }}>
                  <strong style={{ width: 160 }}>{v.id}</strong>
                  <Badge tone={v.kind === "release" ? "accent" : v.kind === "snapshot" ? "warning" : undefined}>{v.kind}</Badge>
                  {v.id === list.latestRelease && <Badge tone="accent">Latest release</Badge>}
                  {fabric?.has(v.id) && <Badge>Fabric</Badge>}
                  {quilt?.has(v.id) && <Badge>Quilt</Badge>}
                  {v.installed && <Badge tone="info">Downloaded</Badge>}
                  <span className="grow small faint">{relativeTime(v.releaseTime)}</span>
                  <Button size="sm" onClick={() => setCreate(v.id)}>Create profile</Button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
      {tab === "loaders" && (
        <div className="grid cols-2">
          <div className="card"><h3>Fabric</h3><p className="dim mt">Primary loader. Installed automatically from meta.fabricmc.net and pinned per profile. Supports {fabric ? fabric.size : "…"} Minecraft versions.</p></div>
          <div className="card"><h3>Quilt</h3><p className="dim mt">Installed automatically from meta.quiltmc.org. Most Fabric mods also run on Quilt. Supports {quilt ? quilt.size : "…"} Minecraft versions.</p></div>
          <div className="card">
            <h3>Forge / NeoForge</h3>
            <p className="dim mt">These loaders install by running their own installer program, which Shadow Client does not execute. Use the official installer, then add the resulting folder under Profiles → Add existing installation. Launching such installs is experimental (Settings → Advanced).</p>
            <div className="row mt">
              <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl("https://neoforged.net/"))}>NeoForge</Button>
              <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl("https://files.minecraftforge.net/"))}>Forge</Button>
            </div>
          </div>
          <div className="card"><h3>Safe version changes</h3><p className="dim mt">A profile with mods is never moved to another Minecraft version automatically. Create a new profile for the new version instead; old profiles and their worlds stay as they are.</p></div>
        </div>
      )}
      {tab === "java" && (
        <div className="grid cols-2">
          <div className="card">
            <div className="card-head"><h3>Detected runtimes</h3><div className="actions"><Button size="sm" variant="ghost" icon={<RefreshCw size={14} />} onClick={async () => setJavas(await api.listJava())}>Rescan</Button></div></div>
            {!javas && <Spinner />}
            {javas?.length === 0 && <p className="dim">No Java found. The required official runtime is downloaded automatically when you launch (after asking).</p>}
            {javas?.map((j) => (
              <div key={j.path} className="row mb" style={{ alignItems: "flex-start" }}>
                <Coffee size={16} />
                <div className="grow"><strong>Java {j.major}</strong> <span className="small faint">{j.version} · {j.vendor ?? "unknown vendor"} · {j.arch} · {j.source}</span><div className="small faint ellipsis">{j.path}</div></div>
              </div>
            ))}
            <p className="small faint">Each Minecraft version declares the Java it needs; the launcher picks a matching runtime per profile (Mojang's runtime first).</p>
          </div>
          <div className="card">
            <h3>Official Mojang runtimes</h3>
            <p className="dim mt mb">Downloaded from Mojang's runtime manifest; every file is SHA-1 verified.</p>
            {RUNTIMES.map((r) => (
              <div className="row mb" key={r.component}>
                <span className="grow">{r.label}</span>
                <Button size="sm" icon={<Download size={14} />} onClick={async () => { const j = await run(() => api.downloadJava(r.component), "Runtime installed"); if (j) setJavas(await api.listJava()); }}>Install</Button>
              </div>
            ))}
          </div>
        </div>
      )}
      {create && <NewProfileDialog initialVersion={create} initialKind="performance" onClose={() => setCreate(null)} />}
    </div>
  );
}
