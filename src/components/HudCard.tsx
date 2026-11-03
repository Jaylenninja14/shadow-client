import { useCallback, useEffect, useState } from "react";
import { Keyboard } from "lucide-react";
import { api, type HudModule } from "../api";
import { useStore } from "../store";
import { Alert, Badge, Button, Spinner, Toggle } from "./ui";

// Lunar-style HUD chooser for Shadow Menu's own HUD. The preview mirrors the mod's default layout
// (client-mod/src/main/java/com/shadowclient/menu/hud/): text modules and keystrokes stacked
// top-left, potion effects and armor on the right edge.

const RIGHT: Record<string, number> = { "potion-effects": 30, "armor-status": 58 };
const PRESETS: { label: string; keys: string[] }[] = [
  { label: "PvP", keys: ["fps", "cps", "keystrokes", "ping", "armor-status", "potion-effects"] },
  { label: "Minimal", keys: ["fps", "cps"] },
  { label: "Survival", keys: ["fps", "coordinates", "armor-status", "potion-effects", "direction", "clock"] },
  { label: "None", keys: [] },
];

export function HudCard({ profileId, running }: { profileId: string; running: boolean }) {
  const { run, toast } = useStore();
  const [modules, setModules] = useState<HudModule[] | null>(null);
  const [on, setOn] = useState<Set<string>>(new Set());
  const [saved, setSaved] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);
  const [rightCps, setRightCps] = useState(true);
  const [savedRightCps, setSavedRightCps] = useState(true);

  const load = useCallback(async () => {
    try {
      const s = await api.hudState(profileId);
      const enabled = new Set(s.modules.filter((m) => m.enabled).map((m) => m.key));
      setModules(s.modules);
      setOn(enabled);
      setSaved(enabled);
      setRightCps(s.rightClickCps);
      setSavedRightCps(s.rightClickCps);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [profileId]);
  useEffect(() => { load(); }, [load]);

  const dirty = on.size !== saved.size || [...on].some((k) => !saved.has(k)) || rightCps !== savedRightCps;
  const flip = (key: string, v: boolean) => setOn((prev) => { const n = new Set(prev); if (v) n.add(key); else n.delete(key); return n; });
  const save = () => run(async () => {
    await api.hudApply(profileId, [...on], on.has("cps") ? rightCps : null);
    await load();
    toast("success", "HUD saved", "It shows the next time this profile starts.");
  });

  const chosen = (modules ?? []).filter((m) => on.has(m.key));
  const left = chosen.filter((m) => !(m.key in RIGHT) && m.key !== "keystrokes");
  const right = chosen.filter((m) => m.key in RIGHT);
  const sample = (m: HudModule) => (m.key === "cps" ? (rightCps ? "12 | 9 CPS" : "12 CPS") : m.key === "fps" ? "240 FPS" : m.key === "ping" ? "18 ms" : m.name);

  return (
    <div className="card mt">
      <div className="card-head">
        <h3>HUD mods</h3>
        <span className="small faint">Choose what shows on screen, like Lunar's mod menu</span>
        <div className="actions"><Badge tone="accent"><Keyboard size={12} /> In game: Right Shift</Badge></div>
      </div>
      {error && <Alert tone="error">{error}</Alert>}
      {!modules && !error && <Spinner />}
      {modules && (
        <div className="row" style={{ alignItems: "flex-start", gap: 18, flexWrap: "wrap" }}>
          <div className="hud-preview" aria-label="HUD preview">
            <div className="hud-left">
              {left.map((m) => <span key={m.key} className="hud-chip">{sample(m)}</span>)}
              {on.has("keystrokes") && (
                <span className="hud-chip hud-keys"><b>W</b><br /><b>A</b><b>S</b><b>D</b><br /><b className="wide">LMB</b><b className="wide">RMB</b></span>
              )}
            </div>
            {right.map((m) => (
              <span key={m.key} className="hud-chip hud-right" style={{ top: `${RIGHT[m.key]}%` }}>{m.name}</span>
            ))}
            <div className="hud-hotbar" />
          </div>
          <div className="col grow" style={{ minWidth: 280, gap: 8 }}>
            <div className="row wrap" style={{ gap: 6 }}>
              <span className="small dim">Presets:</span>
              {PRESETS.map((p) => <Button key={p.label} size="sm" variant="ghost" disabled={running} onClick={() => setOn(new Set(p.keys))}>{p.label}</Button>)}
            </div>
            <div className="grid cols-2" style={{ gap: 6 }}>
              {modules.map((m) => (
                <label key={m.key} className="row" style={{ alignItems: "flex-start" }} title={m.description}>
                  <Toggle on={on.has(m.key)} disabled={running} onChange={(v) => flip(m.key, v)} label={m.name} />
                  <div className="grow"><strong className="small">{m.name}</strong><div className="small faint">{m.description}</div></div>
                </label>
              ))}
            </div>
            {on.has("cps") && (
              <label className="row small">
                <Toggle on={rightCps} disabled={running} onChange={setRightCps} label="Right-click CPS" /> CPS also counts right clicks (shown as left | right)
              </label>
            )}
            <div className="row wrap">
              <Button variant="primary" disabled={running || !dirty} onClick={save}>Save HUD</Button>
              {dirty && <Button variant="ghost" onClick={() => { setOn(new Set(saved)); setRightCps(savedRightCps); }}>Undo changes</Button>}
            </div>
            <p className="small faint">
              In game, press <strong>Right Shift</strong> for the mod menu, then <strong>Edit HUD</strong> in its top bar: drag modules where you want them
              (they snap to the edges, the centre and each other), drag a corner or scroll to resize, and right-click one for its settings (colours,
              background, brackets, keystroke layout…). Positions you set there are kept when you save here.
            </p>
            {running && <p className="small" style={{ color: "var(--warning)" }}>Close the game to change the HUD here, or use the mod menu (Right Shift) in game.</p>}
          </div>
        </div>
      )}
    </div>
  );
}
