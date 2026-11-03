import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { Copy, Edit3, FolderOpen, Link2, Plus, Share2, Trash2, Upload, Wrench } from "lucide-react";
import { api, type DetectedInstall, type JavaInstall, type Profile } from "../api";
import { NewProfileDialog } from "../components/NewProfileDialog";
import { ShareDialog, ShareImportDialog } from "../components/ShareDialogs";
import { Alert, Badge, Button, Confirm, Field, Modal, Select, Spinner, Toggle } from "../components/ui";
import { duration, loaderLabel, presetLabel, relativeTime } from "../format";
import { useStore } from "../store";

export function Profiles() {
  const { profiles, selected, selectProfile, refreshProfiles, run, subPage, games, settings } = useStore();
  const [create, setCreate] = useState(false);
  const [edit, setEdit] = useState<Profile | null>(null);
  const [dup, setDup] = useState<Profile | null>(null);
  const [del, setDel] = useState<Profile | null>(null);
  const [importOpen, setImportOpen] = useState(subPage === "import");
  const [version, setVersion] = useState<Profile | null>(null);
  const [share, setShare] = useState<Profile | null>(null);
  const [fromLink, setFromLink] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const exportProfile = async (p: Profile) => {
    const file = `${p.name.replace(/[<>:"/\|?*]/g, "_")}.mrpack`;
    const dest = await save({ defaultPath: settings?.downloadDir ? `${settings.downloadDir}\\${file}` : file, filters: [{ name: "Modrinth modpack", extensions: ["mrpack"] }] });
    if (!dest) return;
    const r = await run(() => api.exportProfile(p.id, dest, true), "Profile exported");
    if (r) setMessage(`Exported to ${r}. Mods from Modrinth are referenced by URL + hash; other mods and configs are included. Worlds are not included.`);
  };

  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Profiles</h1><div className="sub">Each profile has its own version, loader, mods, settings and game folder.</div></div>
        <div className="actions">
          <Button icon={<Link2 size={14} />} onClick={() => setFromLink(true)}>Add from share link</Button>
          <Button icon={<Upload size={14} />} onClick={() => setImportOpen(true)}>Add existing profile</Button>
          <Button variant="primary" icon={<Plus size={14} />} onClick={() => setCreate(true)}>New profile</Button>
        </div>
      </div>
      {message && <div className="mb"><Alert tone="info" action={<Button size="sm" variant="ghost" onClick={() => setMessage(null)}>Dismiss</Button>}>{message}</Alert></div>}
      <div className="grid auto">
        {profiles.map((p) => (
          <div className="card" key={p.id} style={{ borderColor: p.id === selected?.id ? "var(--accent)" : undefined }}>
            <div className="row">
              <h3 className="grow ellipsis">{p.name}</h3>
              {games[p.id]?.state === "running" && <Badge tone="accent">Running</Badge>}
              {p.external && <Badge tone="info">Existing folder</Badge>}
            </div>
            <div className="row wrap mt">
              <Badge>{p.kind}</Badge>
              <Badge tone="accent">{p.mcVersion}</Badge>
              <Badge>{loaderLabel(p.loader)}{p.loaderVersion ? ` ${p.loaderVersion}` : ""}</Badge>
              <Badge>{presetLabel(p.preset)}</Badge>
            </div>
            <div className="small faint mt">Last played {relativeTime(p.lastPlayed)} · {duration(p.totalPlaySecs)} played{p.notes ? ` · ${p.notes}` : ""}</div>
            <div className="small faint ellipsis" title={p.gameDir ?? "Managed instance folder"}>{p.gameDir ?? "Managed instance folder"}</div>
            <div className="row wrap mt">
              {p.id !== selected?.id && <Button size="sm" variant="primary" onClick={() => selectProfile(p.id)}>Select</Button>}
              <Button size="sm" icon={<Edit3 size={14} />} onClick={() => setEdit(p)}>Edit</Button>
              <Button size="sm" icon={<Copy size={14} />} title="Duplicate" onClick={() => setDup(p)} />
              <Button size="sm" icon={<Link2 size={14} />} onClick={() => setShare(p)}>Share</Button>
              <Button size="sm" icon={<Share2 size={14} />} title="Export as .mrpack" onClick={() => exportProfile(p)} />
              <Button size="sm" icon={<FolderOpen size={14} />} title="Open folder" onClick={() => run(() => api.openProfileFolder(p.id))} />
              <Button size="sm" icon={<Wrench size={14} />} title="Change version" onClick={() => setVersion(p)} />
              <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Delete" onClick={() => setDel(p)} />
            </div>
          </div>
        ))}
      </div>
      {create && <NewProfileDialog onClose={() => { setCreate(false); refreshProfiles(); }} />}
      {edit && <EditProfile profile={edit} onClose={() => { setEdit(null); refreshProfiles(); }} />}
      {dup && <DuplicateDialog profile={dup} onClose={() => { setDup(null); refreshProfiles(); }} />}
      {importOpen && <ImportDialog onClose={() => { setImportOpen(false); refreshProfiles(); }} />}
      {share && <ShareDialog profile={share} onClose={() => setShare(null)} />}
      {fromLink && <ShareImportDialog onClose={() => { setFromLink(false); refreshProfiles(); }} />}
      {version && <VersionDialog profile={version} onClose={() => { setVersion(null); refreshProfiles(); }} onCreate={() => { setVersion(null); setCreate(true); }} />}
      {del && (
        <Confirm title={`Delete “${del.name}”?`} confirmLabel="Delete profile" danger onCancel={() => setDel(null)} onConfirm={async () => {
          const r = await run(() => api.deleteProfile(del.id));
          setDel(null);
          await refreshProfiles();
          if (r) setMessage(r.message);
        }}>
          {del.external ? (
            <p>This profile uses an existing folder (<code>{del.gameDir}</code>). Only the profile entry is removed; <strong>no files in that folder are touched</strong>.</p>
          ) : (
            <>
              <p>Worlds (<code>saves</code>) and screenshots are moved to <em>preserved-worlds</em> in the launcher data folder — they are never deleted.</p>
              <p className="mt">The rest of the instance folder (mods, configs) goes to the Windows Recycle Bin, so it can still be recovered.</p>
            </>
          )}
        </Confirm>
      )}
    </div>
  );
}

function EditProfile({ profile, onClose }: { profile: Profile; onClose: () => void }) {
  const [p, setP] = useState<Profile>(profile);
  const [javas, setJavas] = useState<JavaInstall[]>([]);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => { api.listJava().then(setJavas).catch(() => undefined); }, []);
  const num = (v: string) => (v.trim() === "" ? null : Number(v));
  return (
    <Modal wide title={`Edit ${profile.name}`} onClose={onClose} footer={<>
      <Button onClick={onClose}>Cancel</Button>
      <Button variant="primary" onClick={async () => { try { await api.updateProfile(p); onClose(); } catch (e) { setErr(e instanceof Error ? e.message : String(e)); } }}>Save</Button>
    </>}>
      {err && <div className="mb"><Alert tone="error">{err}</Alert></div>}
      <div className="form-grid">
        <Field label="Name" full><input className="input" value={p.name} maxLength={48} onChange={(e) => setP({ ...p, name: e.target.value })} /></Field>
        <Field label="Java runtime" hint="Automatic picks the version this Minecraft release needs." full>
          <select className="select" value={p.javaPath ?? ""} onChange={(e) => setP({ ...p, javaPath: e.target.value || null })}>
            <option value="">Automatic (recommended)</option>
            {javas.map((j) => <option key={j.path} value={j.path}>Java {j.major} — {j.source} — {j.path}</option>)}
          </select>
        </Field>
        <Field label="Minimum memory (MB)" hint="Empty = automatic"><input className="input" type="number" value={p.memoryMinMb ?? ""} onChange={(e) => setP({ ...p, memoryMinMb: num(e.target.value) })} /></Field>
        <Field label="Maximum memory (MB)" hint="Empty = automatic recommendation"><input className="input" type="number" value={p.memoryMaxMb ?? ""} onChange={(e) => setP({ ...p, memoryMaxMb: num(e.target.value) })} /></Field>
        <Field label="Extra JVM arguments" hint="Overrides the global setting for this profile. Only -XX:, -D, -Xss and similar flags are accepted." full>
          <input className="input mono" value={p.jvmArgs ?? ""} onChange={(e) => setP({ ...p, jvmArgs: e.target.value || null })} />
        </Field>
        <Field label="Window width"><input className="input" type="number" value={p.resolutionWidth ?? ""} onChange={(e) => setP({ ...p, resolutionWidth: num(e.target.value) })} /></Field>
        <Field label="Window height"><input className="input" type="number" value={p.resolutionHeight ?? ""} onChange={(e) => setP({ ...p, resolutionHeight: num(e.target.value) })} /></Field>
        <Field label="Fullscreen">
          <Select value={p.fullscreen === null ? "default" : p.fullscreen ? "on" : "off"} onChange={(v) => setP({ ...p, fullscreen: v === "default" ? null : v === "on" })} options={[{ value: "default", label: "Use global setting" }, { value: "on", label: "Fullscreen" }, { value: "off", label: "Windowed" }]} />
        </Field>
        <Field label="Graphics preset" hint="Applied on first start; use the Performance page to apply to an existing options.txt.">
          <Select value={p.preset ?? ""} onChange={(v) => setP({ ...p, preset: v || null })} options={[{ value: "", label: "Leave settings alone" }, { value: "low", label: "Low-end" }, { value: "balanced", label: "Balanced" }, { value: "high", label: "High FPS" }, { value: "custom", label: "Custom" }]} />
        </Field>
        <Field label="Notes" full><textarea className="textarea" rows={2} value={p.notes ?? ""} onChange={(e) => setP({ ...p, notes: e.target.value || null })} /></Field>
      </div>
    </Modal>
  );
}

function DuplicateDialog({ profile, onClose }: { profile: Profile; onClose: () => void }) {
  const { run } = useStore();
  const [name, setName] = useState(`${profile.name} copy`.slice(0, 48));
  const [worlds, setWorlds] = useState(false);
  return (
    <Modal title={`Duplicate ${profile.name}`} onClose={onClose} footer={<>
      <Button onClick={onClose}>Cancel</Button>
      <Button variant="primary" onClick={async () => { const r = await run(() => api.duplicateProfile(profile.id, name, worlds), "Profile duplicated"); if (r) onClose(); }}>Duplicate</Button>
    </>}>
      <Field label="New name"><input className="input" value={name} maxLength={48} onChange={(e) => setName(e.target.value)} /></Field>
      <label className="check-row mt"><Toggle on={worlds} onChange={setWorlds} label="Copy worlds" /> Also copy worlds</label>
      <p className="small dim">Mods, configs, resource packs and settings are copied into a new isolated instance. The original stays untouched.</p>
    </Modal>
  );
}

function ImportDialog({ onClose }: { onClose: () => void }) {
  const { run } = useStore();
  const [installs, setInstalls] = useState<DetectedInstall[] | null>(null);
  useEffect(() => { api.detectInstalls().then(setInstalls).catch(() => setInstalls([])); }, []);
  // the existing profile's mods, configs and packs are copied into a new Shadow Client profile with the same name
  const add = async (inst: DetectedInstall, gameDir: string, versionId: string | null, name: string) => {
    const r = await run(() => api.importInstall({ root: inst.root, gameDir, versionId, mcVersion: inst.instanceVersion, loader: inst.instanceLoader, name: name.slice(0, 40), mode: "copy", includeWorlds: false }), `${name} added`);
    if (r) onClose();
  };
  return (
    <Modal wide title="Add an existing profile" onClose={onClose}>
      <p className="dim mb">Pick a profile from another launcher: a new Shadow Client profile with the same name is made, and its mods, configs and resource packs are copied into it. Worlds stay where they are and the original isn't changed. Folders to scan can be added under Settings → Minecraft.</p>
      {!installs && <div className="row"><Spinner /> Scanning (read-only)…</div>}
      {installs?.length === 0 && <Alert tone="info">No installations found.</Alert>}
      {installs?.map((inst) => (
        <div className="card" key={inst.root}>
          <div className="row"><strong className="grow">{inst.label}</strong><span className="small faint ellipsis">{inst.root}</span></div>
          {inst.notes.map((n, i) => <div key={i} className="small faint mt">{n}</div>)}
          <div className="list mt">
            {(inst.launcherProfiles.length > 0 ? inst.launcherProfiles.map((lp) => ({ name: lp.name, dir: lp.resolvedGameDir, ver: lp.lastVersionId })) : inst.gameDirs.map((g) => ({ name: inst.label.replace(/^.*?: /, ""), dir: g.path, ver: null as string | null }))).map((row) => {
              const contents = inst.gameDirs.find((g) => g.path === row.dir);
              return (
                <div className="list-row" key={row.name + row.dir}>
                  <div className="grow">
                    <strong>{row.name}</strong> <span className="small faint">{row.ver ?? inst.instanceVersion ?? ""} {inst.instanceLoader ?? ""}</span>
                    <div className="small faint">{contents ? `${contents.mods.length} mods · ${contents.worlds.length} worlds · ${contents.resourcepacks.length} packs` : ""} · {row.dir}</div>
                  </div>
                  <Button size="sm" variant="primary" onClick={() => add(inst, row.dir, row.ver, row.name)}>Add to Shadow Client</Button>
                </div>
              );
            })}
          </div>
        </div>
      ))}
    </Modal>
  );
}

function VersionDialog({ profile, onClose, onCreate }: { profile: Profile; onClose: () => void; onCreate: () => void }) {
  const { run } = useStore();
  const [versions, setVersions] = useState<string[]>([]);
  const [mc, setMc] = useState(profile.mcVersion);
  const [check, setCheck] = useState<{ ok: boolean; unavailable: string[]; unknown: string[]; message: string } | null>(null);
  const [registered, setRegistered] = useState<string | null>(null);
  useEffect(() => { api.listVersions(false, false).then((v) => setVersions(v.versions.map((x) => x.id))); }, []);
  return (
    <Modal title={`Version & launcher integration — ${profile.name}`} onClose={onClose}>
      <h3>Change Minecraft version</h3>
      <p className="small dim mt mb">Profiles with mods are never switched in place: if any mod lacks a build for the new version you'll be offered a new profile instead. Worlds are backed up before a vanilla profile changes version.</p>
      <div className="row">
        <select className="select grow" value={mc} onChange={(e) => { setMc(e.target.value); setCheck(null); }}>{versions.map((v) => <option key={v}>{v}</option>)}</select>
        <Button onClick={async () => setCheck((await run(() => api.checkVersionChange(profile.id, mc))) ?? null)}>Check</Button>
      </div>
      {check && (
        <div className="mt">
          <Alert tone={check.ok ? "success" : "warning"} title={check.message}>
            {check.unavailable.length > 0 && `No build for ${mc}: ${check.unavailable.join(", ")}\n`}
            {check.unknown.length > 0 && `Can't verify (manual files): ${check.unknown.join(", ")}`}
          </Alert>
          <div className="row mt">
            <Button onClick={onCreate}>Create a new profile for {mc}</Button>
            <Button variant="primary" disabled={!check.ok} onClick={async () => { const r = await run(() => api.applyVersionChange(profile.id, mc), "Version changed"); if (r) onClose(); }}>Change this profile</Button>
          </div>
        </div>
      )}
      <div className="sep" />
      <h3>Official launcher</h3>
      <p className="small dim mt mb">Adds this profile to the official Minecraft Launcher the same way the Fabric installer does (launcher_profiles.json is backed up first). Use it if you want to start this profile from the official launcher with its sign-in. With Settings → General → <em>Only show the profile you open</em> on, it becomes the only installation there and the others are put aside until you turn that off.</p>
      <Button disabled={!["vanilla", "fabric", "quilt"].includes(profile.loader)} onClick={async () => { const r = await run(() => api.registerWithOfficial(profile.id)); if (r) setRegistered((r.backup ? `Added as version ${r.versionId}. Backup: ${r.backup}` : `Already there as version ${r.versionId}; nothing else changed.`) + (r.hidden ? ` ${r.hidden} other installation(s) put aside.` : "")); }}>Register with official launcher</Button>
      {registered && <div className="mt"><Alert tone="success">{registered}</Alert></div>}
      <div className="row mt"><Button size="sm" onClick={() => run(() => api.repairProfile(profile.id), "All files verified")}>Verify / repair game files</Button></div>
    </Modal>
  );
}
