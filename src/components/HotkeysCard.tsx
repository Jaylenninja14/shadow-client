import { useCallback, useEffect, useState } from "react";
import { RotateCcw } from "lucide-react";
import { api, type Hotkey } from "../api";
import { keyFromCode, keyFromMouse, keyLabel, UNBOUND } from "../keys";
import { useStore } from "../store";
import { Button, Spinner } from "./ui";

// Rebind the keys PvP players change most. Written to the profile's options.txt
// (backup first), the same place Minecraft's Controls screen saves them.

export function HotkeysCard({ profileId, running, clientKeys }: { profileId: string; running: boolean; clientKeys: boolean }) {
  const { run } = useStore();
  const [keys, setKeys] = useState<Hotkey[] | null | undefined>(undefined);
  const [listening, setListening] = useState<string | null>(null);

  const load = useCallback(async () => setKeys(await api.hotkeysGet(profileId)), [profileId]);
  useEffect(() => { load(); }, [load]);

  const bind = useCallback((id: string, key: string) => run(async () => { await api.hotkeysSet(profileId, id, key); await load(); }), [profileId, run, load]);

  // While listening, the next key press or mouse button becomes the binding (Esc cancels).
  useEffect(() => {
    if (!listening) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const k = e.code === "Escape" ? null : keyFromCode(e.code);
      setListening(null);
      if (k) bind(listening, k);
    };
    const onMouse = (e: MouseEvent) => {
      if (e.button === 0 && e.target instanceof Element && e.target.closest(".hotkey-btn")) return; // clicking a key button again cancels
      e.preventDefault();
      setListening(null);
      bind(listening, keyFromMouse(e.button));
    };
    const noMenu = (e: MouseEvent) => e.preventDefault();
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onMouse, true);
    window.addEventListener("contextmenu", noMenu, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onMouse, true);
      window.removeEventListener("contextmenu", noMenu, true);
    };
  }, [listening, bind]);

  const shown = (keys ?? []).filter((k) => clientKeys || k.group !== "Client");
  const groups = [...new Set(shown.map((k) => k.group))];
  const clash = (k: Hotkey) => k.key !== UNBOUND && shown.some((o) => o.id !== k.id && o.key === k.key);

  return (
    <div className="card mt">
      <div className="card-head"><h3>Hotkeys</h3><span className="small faint">Click a key, then press the new key or mouse button · Esc cancels</span></div>
      {keys === undefined && <Spinner />}
      {keys === null && <p className="dim">Start this profile once so Minecraft creates its settings file; then you can rebind keys here.</p>}
      {keys && (
        <div className="grid cols-2" style={{ gap: 14 }}>
          {groups.map((g) => (
            <div key={g}>
              <div className="small dim mb">{g === "Client" ? "Client (Shadow Menu)" : g}</div>
              {shown.filter((k) => k.group === g).map((k) => (
                <div key={k.id} className="row" style={{ marginBottom: 6 }}>
                  <span className="grow small">{k.label}</span>
                  {clash(k) && <span className="small" style={{ color: "var(--warning)" }} title="Another hotkey uses the same key">same key</span>}
                  <Button
                    size="sm"
                    className="hotkey-btn"
                    variant={listening === k.id ? "primary" : "default"}
                    disabled={running}
                    onClick={() => setListening(listening === k.id ? null : k.id)}
                  >
                    {listening === k.id ? "Press a key…" : keyLabel(k.key)}
                  </Button>
                  <Button size="sm" variant="ghost" icon={<RotateCcw size={13} />} title={`Reset to ${keyLabel(k.defaultKey)}`} disabled={running || k.key === k.defaultKey} onClick={() => bind(k.id, k.defaultKey)} />
                </div>
              ))}
            </div>
          ))}
        </div>
      )}
      {running && <p className="small mt" style={{ color: "var(--warning)" }}>Close the game to change hotkeys here (Minecraft rewrites its settings when it exits), or use Options → Controls in game.</p>}
      <p className="small faint mt">Saved in this profile's options.txt, the same place as Options → Controls in game; a backup is taken before each change.</p>
    </div>
  );
}
