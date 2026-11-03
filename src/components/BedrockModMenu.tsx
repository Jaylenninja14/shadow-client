import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Aperture, ArrowDownToLine, ArrowUpDown, AudioLines, BadgeCheck, Blocks, Camera, Captions, ChevronLeft, ChevronRight, ChevronsUp, Clapperboard, Cloud,
  CloudRain, Copy, Cpu, Crosshair, Disc3, Droplets, Eye, EyeOff, Flame, Footprints, Gamepad2, Gauge, Globe, Grid3x3, Hand, HeartCrack, Image, Info,
  Keyboard, Layers, LayoutGrid, Leaf, Lightbulb, Maximize, MessageSquare, MessageSquareText, Monitor, Moon, Mountain, Mouse, MousePointer2,
  MousePointerClick, MoveVertical, Music, Palette, Pause, PersonStanding, Rabbit, RefreshCw, RotateCcw, RotateCw, Save, Scaling, ScanEye, Search,
  ShieldCheck, Skull, Smile, Sparkle, Sparkles, Speech, Square, SquareDashed, Sun, Sunset, Tag, Telescope, Users, Vibrate, Volume2, Waves, Wifi, ZapOff,
  ZoomIn, Timer, CloudOff, DoorOpen, WandSparkles, Smartphone, Type, ALargeSmall, AtSign, type LucideIcon,
} from "lucide-react";
import { api, type BedrockMenu, type BedrockModule } from "../api";
import { Alert, Button, Confirm, Spinner } from "./ui";
import { useStore } from "../store";

// Bedrock's mod menu, laid out like the in-game Right Shift menu. Bedrock can't load mods, so
// each card is either one of Bedrock's own settings or a part of Shadow Client's resource pack.

const ICONS: Record<string, LucideIcon> = {
  monitor: Monitor, mountain: Mountain, "mouse-pointer-click": MousePointerClick, cpu: Cpu, sparkles: Sparkles, leaf: Leaf, lightbulb: Lightbulb,
  sunset: Sunset, cloud: Cloud, droplets: Droplets, "scan-eye": ScanEye, "zoom-in": ZoomIn, "heart-crack": HeartCrack, vibrate: Vibrate,
  footprints: Footprints, "arrow-down-to-line": ArrowDownToLine, crosshair: Crosshair, flame: Flame, "move-vertical": MoveVertical, sun: Sun,
  "person-standing": PersonStanding, "square-dashed": SquareDashed, tag: Tag, eye: Eye, square: Square, "zap-off": ZapOff, maximize: Maximize,
  "chevrons-up": ChevronsUp, hand: Hand, save: Save, "volume-2": Volume2, "audio-lines": AudioLines, music: Music, waves: Waves, blocks: Blocks,
  skull: Skull, rabbit: Rabbit, users: Users, "cloud-rain": CloudRain, "disc-3": Disc3, speech: Speech, mouse: Mouse, telescope: Telescope,
  "arrow-up-down": ArrowUpDown, keyboard: Keyboard, camera: Camera, pause: Pause, gauge: Gauge, "grid-3x3": Grid3x3, scaling: Scaling, image: Image,
  aperture: Aperture, moon: Moon, sparkle: Sparkle, "mouse-pointer-2": MousePointer2, clapperboard: Clapperboard, "rotate-cw": RotateCw, palette: Palette,
  "eye-off": EyeOff, layers: Layers, "message-square-text": MessageSquareText, "message-square": MessageSquare, info: Info, smile: Smile, copy: Copy,
  captions: Captions, wifi: Wifi, "gamepad-2": Gamepad2, globe: Globe, "shield-check": ShieldCheck, "badge-check": BadgeCheck,
  timer: Timer, "cloud-off": CloudOff, "door-open": DoorOpen, "wand-sparkles": WandSparkles, smartphone: Smartphone, type: Type,
  "a-large-small": ALargeSmall, "at-sign": AtSign,
};

const TABS = [
  { id: "all", label: "All" },
  { id: "performance", label: "Performance" },
  { id: "pvp", label: "PvP" },
  { id: "visual", label: "Visual" },
  { id: "hud", label: "HUD" },
  { id: "audio", label: "Audio" },
  { id: "game", label: "Game" },
  { id: "privacy", label: "Privacy" },
] as const;
type TabId = (typeof TABS)[number]["id"];

export function BedrockModMenu({ initialQuery }: { initialQuery?: string }) {
  const { run } = useStore();
  const [menu, setMenu] = useState<BedrockMenu | null>(null);
  const [user, setUser] = useState<string | null>(null);
  const [tab, setTab] = useState<TabId>("all");
  const [query, setQuery] = useState(initialQuery ?? "");
  const [busy, setBusy] = useState<string | null>(null);
  const [restore, setRestore] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (u: string | null) => {
    try {
      const m = await api.bedrockMenu(u);
      setMenu(m);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => { load(user); }, [load, user]);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (menu?.modules ?? []).filter((m) => (tab === "all" || m.tab === tab) && (!q || m.name.toLowerCase().includes(q)));
  }, [menu, tab, query]);

  const set = async (m: BedrockModule, value: string) => {
    setBusy(m.id);
    const updated = await run(() => api.bedrockMenuSet(menu?.user ?? null, m.id, value));
    setBusy(null);
    if (updated) {
      setMenu((cur) => cur && { ...cur, modules: cur.modules.map((x) => (x.id === updated.id ? updated : x)) });
      if (m.source === "pack") load(menu?.user ?? null);
    }
  };

  const step = (m: BedrockModule, dir: 1 | -1) => {
    const num = (v: string) => (v.trim() !== "" && Number.isFinite(Number(v)) ? Number(v) : null);
    const i = m.choices.findIndex((c) => c.value === m.value || (num(c.value) !== null && num(c.value) === num(m.value)));
    let next = m.choices[i < 0 ? 0 : (i + dir + m.choices.length) % m.choices.length];
    const cur = num(m.value);
    if (i < 0 && cur !== null && m.choices.every((c) => num(c.value) !== null)) {
      // a value between the steps (set in Bedrock itself): go to the nearest step in that direction
      const sorted = [...m.choices].sort((a, b) => num(a.value)! - num(b.value)!);
      next = (dir > 0 ? sorted.find((c) => num(c.value)! > cur) : [...sorted].reverse().find((c) => num(c.value)! < cur)) ?? (dir > 0 ? sorted[0] : sorted[sorted.length - 1]);
    }
    set(m, next.value);
  };

  if (error) return <Alert tone="error">{error}</Alert>;
  if (!menu) return <Spinner />;
  if (!menu.user) return <Alert tone="info">Start Bedrock once and open its settings, so it saves them; then its mod menu works here.</Alert>;
  const locked = menu.running;

  return (
    <div className="bmenu">
      <div className="bmenu-bar">
        <div className="bmenu-pill"><LayoutGrid size={14} /> MOD MENU</div>
        {menu.users.length > 1 && (
          <select className="select" style={{ width: 230 }} value={menu.user ?? ""} onChange={(e) => setUser(e.target.value)} title="Bedrock keeps settings per Xbox account">
            {menu.users.map((u) => <option key={u.id} value={u.id}>{u.label}</option>)}
          </select>
        )}
        <span className="grow" />
        <Button size="sm" variant="ghost" icon={<RefreshCw size={14} />} onClick={() => load(menu.user)}>Refresh</Button>
        {(menu.hasOriginal || menu.packActive) && <Button size="sm" icon={<RotateCcw size={14} />} disabled={locked} onClick={() => setRestore(true)}>Undo all</Button>}
      </div>

      {locked && <div className="mb"><Alert tone="warning">Minecraft Bedrock is running. Close it to change these: it saves its settings when it closes and would undo them.</Alert></div>}

      <div className="bmenu-panel">
        <div className="bmenu-head">
          <div className="bmenu-tabs">
            {TABS.map((t) => (
              <button key={t.id} className={`bmenu-tab${tab === t.id ? " on" : ""}`} onClick={() => setTab(t.id)}>{t.label}</button>
            ))}
          </div>
          <label className="bmenu-search">
            <Search size={13} />
            <input placeholder="Search" value={query} onChange={(e) => setQuery(e.target.value)} />
          </label>
        </div>

        <div className="bmenu-grid">
          {shown.map((m) => {
            const Icon = ICONS[m.icon] ?? Square;
            const off = m.kind === "toggle" && !m.enabled;
            return (
              <div key={m.id} className={`bmenu-card${off ? " off" : ""}`} title={m.about}>
                <div className="bmenu-card-top">
                  <span className="bmenu-name">{m.name}</span>
                  {m.source === "pack" && <span className="bmenu-tag" title="Part of the Shadow Client Modules resource pack">PACK</span>}
                </div>
                <Icon className="bmenu-icon" size={30} strokeWidth={1.6} />
                {!m.available ? (
                  <div className="bmenu-switch na">Not in this version</div>
                ) : m.kind === "toggle" ? (
                  <button className={`bmenu-switch ${m.enabled ? "on" : ""}`} disabled={locked || busy !== null} onClick={() => set(m, m.enabled ? "off" : "on")}>
                    {busy === m.id ? "…" : m.enabled ? "Enabled" : "Disabled"}
                  </button>
                ) : (
                  <div className="bmenu-switch value">
                    <button aria-label="Previous" disabled={locked || busy !== null} onClick={() => step(m, -1)}><ChevronLeft size={14} /></button>
                    <span>{busy === m.id ? "…" : m.valueLabel}</span>
                    <button aria-label="Next" disabled={locked || busy !== null} onClick={() => step(m, 1)}><ChevronRight size={14} /></button>
                  </div>
                )}
              </div>
            );
          })}
          {shown.length === 0 && <div className="faint small">No modules match.</div>}
        </div>
      </div>

      <p className="small faint mt">
        Changes apply the next time Bedrock starts. Settings cards change Bedrock's own options for this account (the file before Shadow Client's first change is kept, so Undo all puts it back).
        PACK cards go into a resource pack called “Shadow Client Modules”, switched on at the top of this account's Global Resources.
      </p>

      {restore && (
        <Confirm title="Undo all Bedrock mod menu changes?" confirmLabel="Undo all" onCancel={() => setRestore(false)} onConfirm={async () => {
          await run(() => api.bedrockMenuRestore(menu.user), "Bedrock settings put back");
          setRestore(false);
          await load(menu.user);
        }}>
          Bedrock's settings for this account go back to how they were before Shadow Client first changed them, and the Shadow Client Modules pack is switched off (its folder is moved to backups).
        </Confirm>
      )}
    </div>
  );
}
