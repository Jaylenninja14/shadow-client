import { useCallback, useEffect, useState } from "react";
import { FolderOpen, Trash2 } from "lucide-react";
import { api, type HardwareInfo, type PackInfo, type ShaderState } from "../api";
import { ProjectBrowser } from "../components/ProjectBrowser";
import { Alert, Badge, Button, Confirm, Empty, Img, Tabs, Toggle } from "../components/ui";
import { bytes } from "../format";
import { useStore } from "../store";

export function ResourcePacks() {
  const { selected, run } = useStore();
  const [tab, setTab] = useState<"installed" | "modrinth" | "curseforge">("installed");
  const [list, setList] = useState<PackInfo[] | null>(null);
  const [remove, setRemove] = useState<PackInfo | null>(null);
  const load = useCallback(async () => { if (selected) setList(await api.resourcepacksList(selected.id)); }, [selected]);
  useEffect(() => { load(); }, [load, tab]);
  if (!selected) return null;
  return (
    <div>
      <Tabs value={tab} onChange={setTab} tabs={[{ id: "installed", label: "Installed" }, { id: "modrinth", label: "Browse Modrinth" }, { id: "curseforge", label: "Browse CurseForge" }]} />
      {tab === "installed" && (
        <>
          <div className="row mb">
            <p className="dim grow">Activating a pack edits the resource pack list in this profile's options.txt (a backup is taken first). High-resolution packs (64x+) cost VRAM and FPS on integrated graphics.</p>
            <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openProfileFolder(selected.id, "resourcepacks"))}>Open folder</Button>
          </div>
          {list && list.length === 0 && <Empty title="No resource packs" />}
          <div className="list">
            {list?.map((p) => (
              <div className="list-row" key={p.fileName}>
                <Img src={p.icon} />
                <div className="grow"><strong>{p.fileName}</strong><div className="small faint">{p.description ?? ""} · {bytes(p.size)}{p.packFormat ? ` · format ${p.packFormat}` : ""}</div></div>
                {p.active && <Badge tone="accent">Active</Badge>}
                <Toggle on={p.active} label="Active" onChange={(v) => run(async () => { await api.resourcepackSetActive(selected.id, p.fileName, v); await load(); })} />
                <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={() => setRemove(p)} />
              </div>
            ))}
          </div>
        </>
      )}
      {tab !== "installed" && <ProjectBrowser platform={tab} kind="resourcepack" />}
      {remove && (
        <Confirm title={`Remove ${remove.fileName}?`} confirmLabel="Remove" danger onCancel={() => setRemove(null)} onConfirm={async () => { await run(() => api.contentRemove(selected.id, remove.fileName, "resourcepack")); setRemove(null); await load(); }}>
          The pack is moved to this profile's backups, not deleted.
        </Confirm>
      )}
    </div>
  );
}

export function Shaders() {
  const { selected, run } = useStore();
  const [tab, setTab] = useState<"installed" | "modrinth" | "curseforge">("installed");
  const [state, setState] = useState<ShaderState | null>(null);
  const [hw, setHw] = useState<HardwareInfo | null>(null);
  const [irisInstalled, setIrisInstalled] = useState<boolean | null>(null);
  const load = useCallback(async () => {
    if (!selected) return;
    setState(await api.shaderState(selected.id));
    const mods = await api.modsList(selected.id, false);
    setIrisInstalled(mods.some((m) => m.meta?.modId === "iris" && m.enabled));
  }, [selected]);
  useEffect(() => { load(); }, [load, tab]);
  useEffect(() => { api.hardwareInfo(false).then(setHw).catch(() => undefined); }, []);
  if (!selected) return null;
  const igpu = hw ? !hw.gpus.some((g) => !g.integrated) : false;
  return (
    <div>
      <Alert tone={igpu ? "warning" : "info"} title="Shaders depend heavily on your GPU">
        {igpu
          ? "This PC uses integrated graphics. Most shader packs will cut FPS sharply; if you try one, choose a light pack such as MakeUp – Ultra Fast. Shaders are never turned on automatically."
          : "Shader packs need Iris (installed with Sodium on Fabric/Quilt). Performance varies a lot between packs; not every pack works with every version."}
      </Alert>
      <div className="mt" />
      <Tabs value={tab} onChange={setTab} tabs={[{ id: "installed", label: "Installed" }, { id: "modrinth", label: "Browse Modrinth" }, { id: "curseforge", label: "Browse CurseForge" }]} />
      {tab === "installed" && (
        <>
          {irisInstalled === false && <div className="mb"><Alert tone="info">Iris is not installed in this profile, so shader packs won't load. Installing a shader from the Browse tab offers Iris automatically when a compatible build exists.</Alert></div>}
          <div className="row mb">
            <span className="grow dim">{state?.enabled ? `Active: ${state.activePack}` : "Shaders are off."}</span>
            {state?.enabled && <Button size="sm" onClick={() => run(async () => { await api.shaderSet(selected.id, null); await load(); }, "Shaders turned off")}>Turn shaders off</Button>}
            <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openProfileFolder(selected.id, "shaderpacks"))}>Open folder</Button>
          </div>
          {state && state.packs.length === 0 && <Empty title="No shader packs" />}
          <div className="list">
            {state?.packs.map((p) => (
              <div className="list-row" key={p.fileName}>
                <div className="grow"><strong>{p.fileName}</strong><div className="small faint">{bytes(p.size)}</div></div>
                {p.active ? <Badge tone="accent">Active</Badge> : <Button size="sm" disabled={!irisInstalled} onClick={() => run(async () => { await api.shaderSet(selected.id, p.fileName); await load(); }, "Shader selected; it loads next time the game starts")}>Use</Button>}
                <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={() => run(async () => { await api.contentRemove(selected.id, p.fileName, "shader"); await load(); })} />
              </div>
            ))}
          </div>
        </>
      )}
      {tab !== "installed" && <ProjectBrowser platform={tab} kind="shader" />}
    </div>
  );
}
