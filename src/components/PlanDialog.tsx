import { useState } from "react";
import { ExternalLink } from "lucide-react";
import { api, type InstallReport, type PlanResult } from "../api";
import { bytes, loaderLabel } from "../format";
import { useStore } from "../store";
import { Alert, Badge, Button, Img, Modal } from "./ui";

/**
 * Shows exactly what an install will add (requested + dependencies), optional extras,
 * conflicts and warnings. Nothing is downloaded until the user confirms.
 */
export function PlanDialog({ result, onClose, onDone, onCreateProfile }: { result: PlanResult; onClose: () => void; onDone: (r: InstallReport) => void; onCreateProfile?: (mc: string) => void }) {
  const { run } = useStore();
  const { plan, planId } = result;
  const [optional, setOptional] = useState<string[]>([]);
  const [report, setReport] = useState<InstallReport | null>(null);
  const close = () => {
    if (!report) api.discardPlan(planId).catch(() => undefined);
    onClose();
  };

  if (report) {
    return (
      <Modal title="Installation finished" onClose={onClose} footer={<Button variant="primary" onClick={onClose}>Done</Button>}>
        {report.installed.length > 0 && <Alert tone="success" title={`Installed ${report.installed.length} file(s)`}>{report.installed.join("\n")}</Alert>}
        {report.skipped.length > 0 && <Alert tone="info" title="Skipped">{report.skipped.join("\n")}</Alert>}
        {report.backups.length > 0 && <Alert tone="info" title="Backed up before replacing">{report.backups.join("\n")}</Alert>}
        {report.warnings.map((w, i) => <Alert key={i} tone="warning">{w}</Alert>)}
        {report.manual.length > 0 && (
          <Alert tone="warning" title="Manual download required">
            {report.manual.map((m) => (
              <div key={m.projectId} className="row mt">
                <span className="grow">{m.title} — the author only allows downloads from CurseForge's website.</span>
                {m.websiteUrl && <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(m.websiteUrl!))}>Open page</Button>}
              </div>
            ))}
          </Alert>
        )}
        {report.restartRequired && <p className="dim mt">If Minecraft is running, restart it to load the changes.</p>}
      </Modal>
    );
  }

  const target = plan.target ? `Minecraft ${plan.target.mcVersion} · ${loaderLabel(plan.target.loader)}` : "";
  return (
    <Modal
      wide
      title="Review installation"
      onClose={close}
      footer={
        <>
          <span className="grow faint small">{target}</span>
          <Button onClick={close}>Cancel</Button>
          <Button
            variant="primary"
            disabled={plan.blocked || plan.files.length === 0}
            onClick={async () => {
              const r = await run(() => api.executePlan(planId, optional));
              if (r) {
                setReport(r);
                onDone(r);
              }
            }}
          >
            Install {plan.files.length > 0 ? `${plan.files.length} file(s)` : ""} {plan.totalSize ? `· ${bytes(plan.totalSize)}` : ""}
          </Button>
        </>
      }
    >
      {plan.blocked && (
        <Alert tone="error" title="Can't install this into the selected profile">
          {plan.blockedReason}
          {plan.suggestion && plan.suggestion.mcVersions.length > 0 && (
            <div className="mt">
              Available for: {plan.suggestion.mcVersions.join(", ")}.
              {onCreateProfile && (
                <div className="row mt wrap">
                  {plan.suggestion.mcVersions.slice(0, 3).map((v) => (
                    <Button key={v} size="sm" onClick={() => onCreateProfile(v)}>Create a {v} profile</Button>
                  ))}
                </div>
              )}
            </div>
          )}
        </Alert>
      )}
      {plan.conflicts.map((c, i) => (
        <Alert key={i} tone={c.severity === "error" ? "error" : "warning"} title={`Conflict: ${c.a} ↔ ${c.b}`}>{c.reason}</Alert>
      ))}
      {plan.files.length > 0 && (
        <>
          <h3 className="mt mb">Files to add</h3>
          <div className="list">
            {plan.files.filter((f) => !f.builtin).map((f) => (
              <div className="list-row" key={f.projectId + f.fileName}>
                <Img src={f.iconUrl} />
                <div className="grow">
                  <div className="row">
                    <strong>{f.title}</strong>
                    <span className="faint small">{f.versionNumber}</span>
                  </div>
                  <div className="small dim ellipsis">{f.fileName}{f.replaces ? ` — replaces ${f.replaces} (backed up)` : ""}</div>
                </div>
                {f.reason === "dependency" && <Badge tone="info">Required by {f.requiredBy}</Badge>}
                {f.reason === "requested" && <Badge tone="accent">Requested</Badge>}
                {f.reason === "update" && <Badge tone="accent">Update</Badge>}
                <span className="small faint">{bytes(f.size)}</span>
              </div>
            ))}
          </div>
          <p className="small faint mt">Each file is downloaded over HTTPS, checked against its published hash, and its own metadata is verified against this profile before it is moved into place.</p>
        </>
      )}
      {plan.optional.length > 0 && (
        <>
          <h3 className="mt mb">Optional extras</h3>
          {plan.optional.map((o) => (
            <label className="check-row" key={o.projectId}>
              <input type="checkbox" checked={optional.includes(o.projectId)} onChange={(e) => setOptional((s) => (e.target.checked ? [...s, o.projectId] : s.filter((x) => x !== o.projectId)))} />
              <span>{o.title}</span>
              <span className="faint small">suggested by {o.requiredBy}</span>
            </label>
          ))}
        </>
      )}
      {plan.alreadyInstalled.length > 0 && <p className="dim mt">Already installed: {plan.alreadyInstalled.join(", ")}</p>}
      {plan.manualDownloads.length > 0 && (
        <Alert tone="warning" title="Needs a manual download">
          {plan.manualDownloads.map((m) => `${m.title}: the author only allows downloads from the CurseForge website.`).join("\n")}
        </Alert>
      )}
      {plan.warnings.map((w, i) => <div className="mt" key={i}><Alert tone="warning">{w}</Alert></div>)}
    </Modal>
  );
}
