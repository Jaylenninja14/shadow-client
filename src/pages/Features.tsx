import { useCallback, useEffect, useState } from "react";
import { CheckCircle2, ExternalLink, Gamepad2, RefreshCw, Sparkles, Trash2 } from "lucide-react";
import { api, type FeatureStatus, type FeaturesView, type GameOption, type PlanResult } from "../api";
import { HotkeysCard } from "../components/HotkeysCard";
import { HudCard } from "../components/HudCard";
import { ServerPluginCard } from "../components/ServerPluginCard";
import { NewProfileDialog } from "../components/NewProfileDialog";
import { PlanDialog } from "../components/PlanDialog";
import { Alert, Badge, Button, Confirm, Empty, Spinner, Toggle } from "../components/ui";
import { loaderLabel } from "../format";
import { useStore } from "../store";

const CATEGORIES = ["HUD", "PvP", "Visual", "Maps", "Utility"];

export function Features() {
  const { selected, run, games } = useStore();
  const [view, setView] = useState<FeaturesView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const [remove, setRemove] = useState<FeatureStatus | null>(null);
  const [createProfile, setCreateProfile] = useState(false);
  const running = selected ? games[selected.id]?.state === "running" : false;

  const load = useCallback(async () => {
    if (!selected) return;
    setError(null);
    try {
      setView(await api.featuresStatus(selected.id));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, [selected]);
  useEffect(() => {
    setView(null);
    load();
  }, [load]);

  if (!selected) return <div className="page"><Empty title="Select or create a profile first" /></div>;
  const modded = selected.loader === "fabric" || selected.loader === "quilt";

  const install = async (keys: string[]) => {
    const r = await run(() => api.featuresPlan(selected.id, keys));
    if (r) setPlan(r);
  };
  const toggle = (f: FeatureStatus, on: boolean) => run(async () => { await api.modSetEnabled(selected.id, f.installedFile!, on); await load(); });

  const packItems = view?.features.filter((f) => f.inPack) ?? [];
  const packMissing = packItems.filter((f) => !f.installedFile && f.available);
  const tiles = view?.features.filter((f) => !f.suite && !f.legacy) ?? [];
  // AxolotlClient and the other mods Shadow Menu replaced, still in this profile
  const leftovers = view?.features.filter((f) => f.legacy && f.installedFile && f.enabled) ?? [];

  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Features</h1>
          <div className="sub">In-game features for {selected.name} · Minecraft {selected.mcVersion} · {loaderLabel(selected.loader)}</div>
        </div>
        <div className="actions"><Button icon={<RefreshCw size={14} />} onClick={load}>Refresh</Button></div>
      </div>

      {!modded && (
        <Alert tone="info" title="In-game features need a Fabric or Quilt profile" action={<Button size="sm" variant="primary" onClick={() => setCreateProfile(true)}>Create PvP profile</Button>}>
          This profile is {loaderLabel(selected.loader)}. Features run as Fabric mods inside the game, so they need a Fabric (or Quilt) profile. Your current profile stays unchanged.
        </Alert>
      )}
      {running && <div className="mb"><Alert tone="info">Minecraft is running. Changes apply after you restart the game.</Alert></div>}
      {error && <Alert tone="error">{error}</Alert>}
      {modded && !view && !error && <div className="row dim"><Spinner /> Checking which features exist for Minecraft {selected.mcVersion}…</div>}

      {modded && view && (
        <div className="hero">
          <div className="row wrap" style={{ alignItems: "flex-start" }}>
            <div className="grow col" style={{ gap: 6, minWidth: 280 }}>
              <div className="row small dim"><Sparkles size={14} /> Built in</div>
              <div className="hero-title">Shadow Client mod menu</div>
              <p className="dim">
                Shadow Client's own in-game client comes with every Fabric and Quilt profile: a movable HUD with a drag-and-drop editor, zoom,
                freelook, motion blur, chat heads, a saturation display, ping in tab, a mod list and the other PvP, Hypixel and visual modules,
                plus the Shadow Client title screen. All made by Shadow Client; nothing to install.
              </p>
              <div className="row wrap" style={{ gap: 4 }}>
                {view.suiteProvides.map((p) => <Badge key={p}>{p}</Badge>)}
              </div>
              {packItems.length > 0 && (
                <div className="small faint mt">
                  Extras: {packItems.map((f) => `${f.mod}${f.installedFile ? " ✓" : f.available === false ? " (not for this version)" : ""}`).join(" · ")}
                </div>
              )}
            </div>
            <div className="col" style={{ alignItems: "flex-end", gap: 8 }}>
              {packMissing.length > 0 ? (
                <Button className="play-btn" variant="primary" icon={<Sparkles size={18} />} onClick={() => install(packMissing.map((f) => f.key))}>Install extras</Button>
              ) : (
                <Badge tone="accent"><CheckCircle2 size={12} /> Built in</Badge>
              )}
            </div>
          </div>
          <div className="mt small dim">
            <Gamepad2 size={13} /> In game: press <strong>Right Shift</strong> for the mod menu (categories, search, and a settings page per module with
            sliders, colours and keys) and <strong>Edit HUD</strong> to drag, resize and snap HUD modules. <strong>Client Settings</strong> in the menu
            change its theme and accent colour.
          </div>
        </div>
      )}

      {modded && view && leftovers.map((f) => (
        <div key={f.key} className="mt">
          <Alert tone="warning" title={`${f.mod} is still in this profile`} action={<Button size="sm" variant="primary" onClick={() => setRemove(f)}>Remove</Button>}>
            {f.note}
          </Alert>
        </div>
      ))}

      {modded && view && <HudCard profileId={selected.id} running={running} />}

      {modded && view && CATEGORIES.map((cat) => {
        const items = tiles.filter((t) => t.category === cat);
        if (items.length === 0) return null;
        return (
          <div key={cat} className="mt">
            <h3 className="mb">{cat} add-ons</h3>
            <div className="grid auto">
              {items.map((f) => <FeatureTile key={f.key} f={f} onInstall={() => install([f.key])} onToggle={(v) => toggle(f, v)} onRemove={() => setRemove(f)} />)}
            </div>
          </div>
        );
      })}

      <HotkeysCard profileId={selected.id} running={running} clientKeys={modded} />

      <GameOptions profileId={selected.id} running={running} />

      <ServerPluginCard />

      <p className="small faint mt">The mod menu and its modules are Shadow Client's own; the optional add-ons above come from open Fabric mods on Modrinth and are installed with the same compatibility and hash checks as any mod. Use only what the server you play on allows — freelook and minimap radars are restricted on some servers.</p>

      {plan && <PlanDialog result={plan} onClose={() => setPlan(null)} onDone={() => load()} />}
      {remove && (
        <Confirm title={`Remove ${remove.name}?`} confirmLabel="Remove" danger onCancel={() => setRemove(null)} onConfirm={async () => { await run(() => api.modRemove(selected.id, remove.installedFile!)); setRemove(null); await load(); }}>
          {remove.mod} is moved to this profile's mod backups, so you can restore it later.
        </Confirm>
      )}
      {createProfile && <NewProfileDialog initialKind="pvp" onClose={() => setCreateProfile(false)} />}
    </div>
  );
}

function FeatureTile({ f, onInstall, onToggle, onRemove }: { f: FeatureStatus; onInstall: () => void; onToggle: (v: boolean) => void; onRemove: () => void }) {
  const { run } = useStore();
  return (
    <div className="card" style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      <div className="row">
        <strong className="grow">{f.name}</strong>
        {f.installedFile ? (
          <Toggle on={f.enabled} onChange={onToggle} label={`${f.name} enabled`} />
        ) : f.available ? (
          <Button size="sm" variant="primary" onClick={onInstall}>Add</Button>
        ) : (
          <Badge>{f.available === false ? "Not available" : "Unknown"}</Badge>
        )}
      </div>
      <div className="small faint">via {f.mod}{f.availableVersion ? ` · ${f.availableVersion}` : ""}</div>
      <div className="small dim">{f.summary}</div>
      {f.overlapsSuite && f.installedFile && f.enabled && <div className="small" style={{ color: "var(--warning)" }}>Shadow Client's mod menu also has this — consider turning one off.</div>}
      {f.coveredBySuite && !f.installedFile && <div className="small faint">Already built into Shadow Client's mod menu.</div>}
      {f.note && <div className="small" style={{ color: "var(--warning)" }}>{f.note}</div>}
      {!f.available && f.checkNote && <div className="small faint">{f.checkNote}</div>}
      <div className="row" style={{ marginTop: "auto" }}>
        <Button size="sm" variant="ghost" icon={<ExternalLink size={13} />} onClick={() => run(() => api.openUrl(`https://modrinth.com/mod/${f.slug}`))}>Mod page</Button>
        <span className="grow" />
        {f.installedFile && <Button size="sm" variant="ghost" icon={<Trash2 size={13} />} title="Remove" onClick={onRemove} />}
      </div>
    </div>
  );
}

function GameOptions({ profileId, running }: { profileId: string; running: boolean }) {
  const { run } = useStore();
  const [opts, setOpts] = useState<GameOption[] | null | undefined>(undefined);
  const load = useCallback(async () => setOpts(await api.gameOptionsGet(profileId)), [profileId]);
  useEffect(() => { load(); }, [load]);
  const set = (key: string, value: string) => run(async () => { await api.gameOptionsSet(profileId, key, value); await load(); });
  return (
    <div className="card mt">
      <div className="card-head"><h3>Game options (no mods needed)</h3><span className="small faint">Minecraft's own settings that PvP clients usually change</span></div>
      {opts === undefined && <Spinner />}
      {opts === null && <p className="dim">Start this profile once so Minecraft creates its settings file; then these switches appear here.</p>}
      {opts && (
        <div className="grid cols-2">
          {opts.filter((o) => o.value !== null).map((o) => (
            <div key={o.key} className="row" style={{ alignItems: "flex-start" }}>
              <div className="grow">
                <strong>{o.label}</strong>
                <div className="small faint">{o.description}</div>
              </div>
              {o.kind === "bool" ? (
                <Toggle on={o.value === "true"} disabled={running} onChange={(v) => set(o.key, String(v))} label={o.label} />
              ) : (
                <select className="select" style={{ width: 90 }} disabled={running} value={String(Math.round(Number(o.value) * 100))} onChange={(e) => set(o.key, String(Number(e.target.value) / 100))}>
                  {[0, 25, 50, 75, 100].map((p) => <option key={p} value={p}>{p}%</option>)}
                  {![0, 25, 50, 75, 100].includes(Math.round(Number(o.value) * 100)) && <option value={Math.round(Number(o.value) * 100)}>{Math.round(Number(o.value) * 100)}%</option>}
                </select>
              )}
            </div>
          ))}
          {opts.every((o) => o.value === null) && <p className="dim">This Minecraft version has none of these options.</p>}
        </div>
      )}
      <p className="small faint mt">A backup of options.txt is taken before each change (Performance → Settings backups).</p>
    </div>
  );
}
