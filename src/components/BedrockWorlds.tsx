import { useCallback, useEffect, useState } from "react";
import { Archive, FolderOpen, Globe2, RotateCcw } from "lucide-react";
import { api, type BedrockWorld, type WorldBackup } from "../api";
import { Alert, Button, Empty, Spinner } from "./ui";
import { useStore } from "../store";

// Bedrock worlds for the account that played last: back one up to a .mcworld in Shadow Client's
// backups, or restore a backup (Bedrock imports it as a new copy; nothing is overwritten).

const mb = (n: number) => `${(n / 1048576).toFixed(n > 10485760 ? 0 : 1)} MB`;

export function BedrockWorlds() {
  const { run } = useStore();
  const [worlds, setWorlds] = useState<BedrockWorld[] | null>(null);
  const [backups, setBackups] = useState<WorldBackup[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setWorlds(await api.bedrockWorlds(null));
      setBackups(await api.bedrockWorldBackups());
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => { load(); }, [load]);

  const backup = async (w: BedrockWorld) => {
    setBusy(w.folder);
    const path = await run(() => api.bedrockWorldBackup(null, w.folder), `Backed up ${w.name}`);
    setBusy(null);
    if (path) load();
  };

  if (error) return <Alert tone="info">{error}</Alert>;
  if (!worlds) return <Spinner />;
  const folder = backups[0]?.path.replace(/[\\/][^\\/]+$/, "");
  return (
    <div>
      {worlds.length === 0 && <Empty title="No Bedrock worlds" icon={<Globe2 size={28} />}>Worlds you create in Bedrock show up here.</Empty>}
      {worlds.length > 0 && (
        <div className="list mb">
          {worlds.map((w) => (
            <div key={w.folder} className="list-row">
              {w.icon ? <img src={w.icon} alt="" width={64} height={36} style={{ objectFit: "cover", borderRadius: 4 }} /> : <div className="server-icon"><Globe2 size={18} /></div>}
              <div className="grow" style={{ minWidth: 0 }}>
                <strong className="ellipsis">{w.name}</strong>
                <div className="small dim">{mb(w.size)}{w.played ? ` · played ${w.played}` : ""}</div>
              </div>
              <Button icon={<Archive size={14} />} disabled={busy !== null} onClick={() => backup(w)}>{busy === w.folder ? "Backing up…" : "Back up"}</Button>
            </div>
          ))}
        </div>
      )}
      <div className="row mb">
        <h3 className="grow">Backups</h3>
        {folder && <Button size="sm" icon={<FolderOpen size={14} />} onClick={() => run(() => api.openPath(folder))}>Open folder</Button>}
      </div>
      {backups.length === 0 && <p className="small faint">No world backups yet.</p>}
      {backups.length > 0 && (
        <div className="list">
          {backups.map((b) => (
            <div key={b.path} className="list-row">
              <div className="grow" style={{ minWidth: 0 }}>
                <strong className="ellipsis">{b.file.replace(/\.mcworld$/, "")}</strong>
                <div className="small dim">{mb(b.size)}{b.created ? ` · ${b.created}` : ""}</div>
              </div>
              <Button icon={<RotateCcw size={14} />} onClick={() => run(() => api.bedrockWorldRestore(b.path), "Bedrock is importing the world as a new copy")}>Restore</Button>
            </div>
          ))}
        </div>
      )}
      <p className="small faint mt">Close Bedrock before backing up. Restoring opens the backup in Bedrock, which imports it as a new world, so your current one stays as it is.</p>
    </div>
  );
}
