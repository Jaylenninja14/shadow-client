import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { ExternalLink, FileUp, Layers, RefreshCw } from "lucide-react";
import { api, type PackInstallReport, type PackSummary, type Profile } from "../api";
import { ProjectBrowser } from "../components/ProjectBrowser";
import { Alert, Badge, Button, Confirm, Empty, Modal, Spinner, Tabs } from "../components/ui";
import { bytes, loaderLabel } from "../format";
import { useStore } from "../store";

type Tab = "modrinth" | "curseforge" | "file" | "installed";

export function Modpacks() {
  const { profiles, refreshProfiles, selectProfile, run } = useStore();
  const [tab, setTab] = useState<Tab>("modrinth");
  const [pending, setPending] = useState<{ platform: "modrinth" | "curseforge"; id: string; title: string } | null>(null);
  const [installing, setInstalling] = useState(false);
  const [result, setResult] = useState<{ profile: Profile; report: PackInstallReport } | null>(null);
  const [file, setFile] = useState<{ path: string; summary: PackSummary } | null>(null);
  const [update, setUpdate] = useState<{ profile: Profile; path: string; summary: PackSummary } | null>(null);
  const [updateReport, setUpdateReport] = useState<PackInstallReport | null>(null);
  const packProfiles = profiles.filter((p) => p.notes?.startsWith("Modpack:"));

  const pickFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Modpacks", extensions: ["mrpack", "zip"] }] });
    if (typeof path !== "string") return null;
    const summary = await run(() => api.modpackInspect(path));
    return summary ? { path, summary } : null;
  };

  const done = async (r: { profile: Profile; report: PackInstallReport } | undefined) => {
    setInstalling(false);
    if (r) {
      setResult(r);
      await refreshProfiles();
      await selectProfile(r.profile.id);
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Modpacks</h1><div className="sub">Every pack installs into its own isolated profile. Worlds and settings of other profiles are never touched.</div></div>
      </div>
      <Tabs<Tab> value={tab} onChange={setTab} tabs={[{ id: "modrinth", label: "Modrinth" }, { id: "curseforge", label: "CurseForge" }, { id: "file", label: "Import file" }, { id: "installed", label: `Installed packs (${packProfiles.length})` }]} />
      {(tab === "modrinth" || tab === "curseforge") && <ProjectBrowser platform={tab} kind="modpack" onInstallPack={(platform, id, title) => setPending({ platform, id, title })} />}
      {tab === "file" && (
        <div className="card">
          <h3>Import a modpack file</h3>
          <p className="dim mt">Supports Modrinth <code>.mrpack</code> and CurseForge export <code>.zip</code> files. The pack is inspected first: paths are validated, downloads are restricted to the hosts the format allows, and every file is hash-checked.</p>
          <div className="row mt">
            <Button variant="primary" icon={<FileUp size={16} />} onClick={async () => { const f = await pickFile(); if (f) setFile(f); }}>Choose file…</Button>
          </div>
        </div>
      )}
      {tab === "installed" && (
        packProfiles.length === 0 ? <Empty title="No modpacks installed" icon={<Layers size={36} />} /> : (
          <div className="list">
            {packProfiles.map((p) => (
              <div className="list-row" key={p.id}>
                <div className="grow"><strong>{p.name}</strong><div className="small faint">{p.notes} · Minecraft {p.mcVersion} · {loaderLabel(p.loader)}</div></div>
                <Button size="sm" icon={<RefreshCw size={14} />} onClick={async () => { const f = await pickFile(); if (f) setUpdate({ profile: p, ...f }); }}>Update from file…</Button>
              </div>
            ))}
          </div>
        )
      )}

      {pending && (
        <Confirm title={`Install ${pending.title}?`} confirmLabel="Install as new profile" onCancel={() => setPending(null)} onConfirm={async () => {
          const p = pending;
          setPending(null);
          setInstalling(true);
          await done(await run(() => api.modpackInstallRemote(p.platform, p.id, null)));
        }}>
          <p>The latest release of the pack is downloaded, verified and installed into a new profile. Large packs can take a few minutes; progress is shown on the Downloads page.</p>
        </Confirm>
      )}
      {file && (
        <Modal title={file.summary.name} onClose={() => setFile(null)} footer={<>
          <Button onClick={() => setFile(null)}>Cancel</Button>
          <Button variant="primary" disabled={!file.summary.supported} onClick={async () => { const f = file; setFile(null); setInstalling(true); await done(await run(() => api.modpackInstallFile(f.path, null))); }}>Install as new profile</Button>
        </>}>
          <PackSummaryView s={file.summary} />
        </Modal>
      )}
      {update && (
        <Modal title={`Update ${update.profile.name}`} onClose={() => setUpdate(null)} footer={<>
          <Button onClick={() => setUpdate(null)}>Cancel</Button>
          <Button variant="primary" onClick={async () => { const u = update; setUpdate(null); const r = await run(() => api.modpackUpdateFile(u.profile.id, u.path)); if (r) setUpdateReport(r); }}>Back up and update</Button>
        </>}>
          <PackSummaryView s={update.summary} />
          <div className="mt"><Alert tone="info">Before updating, mods, config, options.txt and servers.dat are zipped into this profile's backups. Worlds are not touched, and your options.txt / servers.dat are kept. Files the new pack no longer contains are moved to backups.</Alert></div>
        </Modal>
      )}
      {installing && <Modal title="Installing modpack" onClose={() => undefined}><div className="row"><Spinner /> Downloading and verifying files… see Downloads for details.</div></Modal>}
      {result && <ReportModal title={`${result.profile.name} installed`} report={result.report} onClose={() => setResult(null)} />}
      {updateReport && <ReportModal title="Modpack updated" report={updateReport} onClose={() => setUpdateReport(null)} />}
    </div>
  );
}

function PackSummaryView({ s }: { s: PackSummary }) {
  return (
    <div>
      {s.summary && <p className="dim mb">{s.summary}</p>}
      <div className="kv">
        <span>Format</span><span>{s.format === "mrpack" ? "Modrinth (.mrpack)" : "CurseForge (.zip)"}</span>
        <span>Version</span><span>{s.version || "—"}</span>
        <span>Minecraft</span><span>{s.mcVersion}</span>
        <span>Loader</span><span>{loaderLabel(s.loader)} {s.loaderVersion ?? ""}</span>
        <span>Files</span><span>{s.fileCount} to download{s.totalSize ? ` (${bytes(s.totalSize)})` : ""}, {s.overrideCount} included</span>
      </div>
      {!s.supported && <div className="mt"><Badge tone="danger">Not supported</Badge></div>}
      {s.warnings.map((w, i) => <div className="mt" key={i}><Alert tone="warning">{w}</Alert></div>)}
    </div>
  );
}

function ReportModal({ title, report, onClose }: { title: string; report: PackInstallReport; onClose: () => void }) {
  const { run } = useStore();
  return (
    <Modal wide title={title} onClose={onClose} footer={<Button variant="primary" onClick={onClose}>Done</Button>}>
      <Alert tone="success" title={`${report.installedFiles} files downloaded and verified, ${report.overrides} files from the pack applied`}>{report.backup ? `Backup: ${report.backup}` : undefined}</Alert>
      {report.manual.length > 0 && (
        <div className="mt">
          <Alert tone="warning" title={`${report.manual.length} file(s) must be downloaded manually`}>
            The authors of these files don't allow downloads through third-party apps. Download each one from CurseForge and place it in the folder shown, then return here.
          </Alert>
          <div className="list mt">
            {report.manual.map((m) => (
              <div className="list-row" key={m.fileName}>
                <div className="grow"><strong>{m.name}</strong><div className="small faint">{m.fileName} → {m.targetFolder}</div></div>
                <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(m.url))}>Open download page</Button>
                <Button size="sm" onClick={() => run(() => api.openPath(m.targetFolder))}>Open folder</Button>
              </div>
            ))}
          </div>
        </div>
      )}
      {report.skipped.length > 0 && <div className="mt"><Alert tone="info" title="Skipped">{report.skipped.join("\n")}</Alert></div>}
      {report.warnings.map((w, i) => <div className="mt" key={i}><Alert tone="warning">{w}</Alert></div>)}
    </Modal>
  );
}
