import { CheckCircle2, RotateCcw, X, XCircle } from "lucide-react";
import { api } from "../api";
import { Alert, Badge, Button, Empty, Progress } from "../components/ui";
import { bytes, percent, relativeTime } from "../format";
import { useStore } from "../store";

export function Downloads() {
  const { jobs, refreshJobs, run } = useStore();
  const active = jobs.filter((j) => j.status === "running" || j.status === "queued");
  const done = jobs.filter((j) => j.status !== "running" && j.status !== "queued");
  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Downloads</h1><div className="sub">Every file is fetched over HTTPS and verified against its published hash. Interrupted downloads resume where they stopped.</div></div>
        <div className="actions"><Button disabled={done.length === 0} onClick={() => run(async () => { await api.downloadsClear(); await refreshJobs(); })}>Clear finished</Button></div>
      </div>
      <h3 className="mb">Active</h3>
      {active.length === 0 && <p className="dim mb">Nothing downloading right now.</p>}
      {active.map((j) => (
        <div className="card" key={j.id}>
          <div className="row">
            <strong className="grow">{j.title}</strong>
            <span className="small dim">{bytes(j.bytesPerSec)}/s</span>
            <Button size="sm" variant="danger" icon={<X size={14} />} onClick={() => run(() => api.downloadCancel(j.id))}>Cancel</Button>
          </div>
          <div className="mt"><Progress value={j.totalBytes ? percent(j.downloadedBytes, j.totalBytes) : percent(j.doneFiles, j.totalFiles)} /></div>
          <div className="small faint mt">{j.doneFiles} of {j.totalFiles} files · {bytes(j.downloadedBytes)}{j.totalBytes ? ` of ${bytes(j.totalBytes)}` : " (total size unknown)"}{j.skippedFiles ? ` · ${j.skippedFiles} already verified` : ""}</div>
        </div>
      ))}
      <h3 className="mt mb">History</h3>
      {done.length === 0 && <Empty title="No downloads yet" />}
      {done.length > 0 && (
        <div className="list">
          {done.map((j) => (
            <div className="list-row" key={j.id}>
              {j.status === "completed" ? <CheckCircle2 size={18} color="var(--success)" /> : <XCircle size={18} color={j.status === "cancelled" ? "var(--text-faint)" : "var(--danger)"} />}
              <div className="grow" style={{ minWidth: 0 }}>
                <div className="row"><strong className="ellipsis">{j.title}</strong><Badge tone={j.status === "completed" ? "accent" : j.status === "failed" ? "danger" : undefined}>{j.status}</Badge></div>
                <div className="small faint">{j.doneFiles}/{j.totalFiles} files · {bytes(j.downloadedBytes)} · {relativeTime(j.finishedAt ?? j.startedAt)}</div>
                {j.error && <div className="small" style={{ color: "var(--danger)" }}>{j.error}</div>}
                {j.failed.length > 0 && <details className="small faint"><summary>Failed files</summary>{j.failed.join("\n")}</details>}
              </div>
              {j.retryable && j.status !== "completed" && <Button size="sm" icon={<RotateCcw size={14} />} onClick={() => run(() => api.downloadRetry(j.id), "Download completed")}>Retry</Button>}
            </div>
          ))}
        </div>
      )}
      {done.some((j) => j.status === "failed" && !j.retryable) && <div className="mt"><Alert tone="info">Downloads from a previous session can't be resumed from here; start the action (install / launch) again and verified files will be skipped.</Alert></div>}
    </div>
  );
}
