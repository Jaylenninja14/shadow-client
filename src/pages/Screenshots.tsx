import { useCallback, useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import { Camera, ExternalLink, FolderOpen, RefreshCw, Trash2 } from "lucide-react";
import { api, type Screenshot } from "../api";
import { Button, Confirm, Empty, Modal, Spinner } from "../components/ui";
import { useStore } from "../store";

// The selected profile's screenshots (F2 in game), newest first.

export function Screenshots() {
  const { run, selected } = useStore();
  const [shots, setShots] = useState<Screenshot[] | null>(null);
  const [open, setOpen] = useState<Screenshot | null>(null);
  const [remove, setRemove] = useState<Screenshot | null>(null);

  const load = useCallback(async () => {
    if (!selected) return;
    setShots(await api.screenshotsList(selected.id).catch(() => []));
  }, [selected]);
  useEffect(() => { load(); }, [load]);

  const folder = shots?.[0]?.path.replace(/[\\/][^\\/]+$/, "");

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Screenshots</h1>
          <div className="sub">{selected ? `${selected.name} · press F2 in game to take one` : "Select a profile"}</div>
        </div>
        <div className="actions">
          {folder && <Button icon={<FolderOpen size={14} />} onClick={() => run(() => api.openPath(folder))}>Open folder</Button>}
          <Button icon={<RefreshCw size={14} />} onClick={load}>Refresh</Button>
        </div>
      </div>
      {!shots && <Spinner />}
      {shots && shots.length === 0 && <Empty title="No screenshots yet" icon={<Camera size={28} />}>Press F2 in game; they show up here.</Empty>}
      {shots && shots.length > 0 && (
        <div className="shots-grid">
          {shots.map((s) => (
            <button key={s.path} className="shot-tile" onClick={() => setOpen(s)} title={s.name}>
              <img src={convertFileSrc(s.path)} alt={s.name} loading="lazy" />
              <span>{s.taken ?? s.name}</span>
            </button>
          ))}
        </div>
      )}
      {open && (
        <Modal title={open.name} wide onClose={() => setOpen(null)} footer={
          <>
            <Button icon={<ExternalLink size={14} />} onClick={() => run(() => api.openPath(open.path))}>Open</Button>
            <Button variant="ghost" icon={<Trash2 size={14} />} onClick={() => { setRemove(open); setOpen(null); }}>Delete</Button>
            <Button variant="primary" onClick={() => setOpen(null)}>Close</Button>
          </>
        }>
          <img src={convertFileSrc(open.path)} alt={open.name} style={{ width: "100%", borderRadius: 8 }} />
          <p className="small faint mt">{open.taken} · {(open.size / 1048576).toFixed(1)} MB</p>
        </Modal>
      )}
      {remove && selected && (
        <Confirm title={`Delete ${remove.name}?`} confirmLabel="Delete" danger onCancel={() => setRemove(null)} onConfirm={async () => {
          await run(() => api.screenshotRemove(selected.id, remove.name), "Screenshot moved to Shadow Client's backups");
          setRemove(null);
          await load();
        }}>
          It's moved to Shadow Client's backups, not deleted for good.
        </Confirm>
      )}
    </div>
  );
}
