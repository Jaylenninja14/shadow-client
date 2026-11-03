import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowRight, Blocks, Boxes, CornerDownLeft, Layers, Search, Settings as SettingsIcon, type LucideIcon } from "lucide-react";
import { api, type BedrockModule, type InstalledMod } from "../api";
import { loaderLabel } from "../format";
import { useStore, type Page } from "../store";

export interface PaletteTarget {
  id: Page;
  label: string;
  icon: LucideIcon;
  /** extra words people might type for it */
  keywords?: string;
}

interface Item {
  key: string;
  group: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
  words: string;
  go: () => void;
}

const SETTINGS_SECTIONS: { id: string; label: string; words: string }[] = [
  { id: "general", label: "General", words: "theme dark light accent colour color language startup notifications folder official launcher" },
  { id: "minecraft", label: "Minecraft", words: "java memory ram jvm garbage collector window fullscreen resolution snapshots default version loader" },
  { id: "performance", label: "Performance", words: "preset sodium performance mods fps" },
  { id: "downloads", label: "Downloads", words: "parallel downloads speed limit retries cache" },
  { id: "accounts", label: "Accounts & services", words: "microsoft account azure client id curseforge api key" },
  { id: "advanced", label: "Advanced", words: "debug logging benchmark forge neoforge experimental diagnostics data folder" },
];

/** Ctrl+K: jump to any page, profile, setting, installed mod or Bedrock module by typing. */
export function CommandPalette({ targets, onClose }: { targets: PaletteTarget[]; onClose: () => void }) {
  const { profiles, selected, selectProfile, setPage } = useStore();
  const [q, setQ] = useState("");
  const [at, setAt] = useState(0);
  const [bedrock, setBedrock] = useState<BedrockModule[]>([]);
  const [mods, setMods] = useState<InstalledMod[]>([]);
  const list = useRef<HTMLDivElement>(null);

  useEffect(() => {
    api.bedrockMenu(null).then((m) => setBedrock(m.modules)).catch(() => undefined);
    if (selected && selected.loader !== "vanilla") api.modsList(selected.id, false).then((m) => setMods(m.filter((x) => !x.builtin))).catch(() => undefined);
  }, [selected]);

  const items = useMemo(() => {
    const go = (p: Page, sub?: string) => () => { setPage(p, sub); onClose(); };
    const all: Item[] = [
      ...targets.map((t) => ({ key: `page:${t.id}`, group: "Pages", label: t.label, icon: t.icon, words: `${t.label} ${t.keywords ?? ""}`, go: go(t.id) })),
      ...profiles.map((p) => ({
        key: `profile:${p.id}`, group: "Profiles", label: p.name, hint: `${p.mcVersion} ${loaderLabel(p.loader)}${p.id === selected?.id ? " · selected" : ""}`,
        icon: Layers, words: `${p.name} profile ${p.mcVersion} ${p.loader}`,
        go: () => { selectProfile(p.id); setPage("home"); onClose(); },
      })),
      ...SETTINGS_SECTIONS.map((s) => ({ key: `settings:${s.id}`, group: "Settings", label: `Settings: ${s.label}`, icon: SettingsIcon, words: `settings ${s.label} ${s.words}`, go: go("settings", s.id) })),
      ...mods.map((m) => {
        const name = m.meta?.name || m.source?.title || m.identified?.title || m.fileName;
        return { key: `mod:${m.fileName}`, group: `Mods in ${selected?.name ?? "this profile"}`, label: name, hint: m.enabled ? "installed" : "disabled", icon: Boxes, words: `${name} ${m.fileName}`, go: go("mods", `installed|${name}`) };
      }),
      ...bedrock.map((m) => ({ key: `bedrock:${m.id}`, group: "Bedrock modules", label: m.name, hint: m.about, icon: Blocks, words: `${m.name} bedrock ${m.tab}`, go: go("bedrock", `menu|${m.name}`) })),
    ];
    const s = q.trim().toLowerCase();
    if (!s) return all.filter((i) => i.group === "Pages" || i.group === "Profiles");
    const words = s.split(/\s+/);
    const scored = all
      .map((i) => {
        const label = i.label.toLowerCase();
        const hay = `${label} ${i.words.toLowerCase()}`;
        if (!words.every((w) => hay.includes(w))) return null;
        return { i, score: label.startsWith(s) ? 0 : label.includes(s) ? 1 : 2 };
      })
      .filter((x): x is { i: Item; score: number } => x !== null)
      .sort((a, b) => a.score - b.score);
    const out = scored.map((x) => x.i).slice(0, 40);
    out.push({ key: "search:settings", group: "Search", label: `Search settings for “${q.trim()}”`, icon: Search, words: "", go: go("settings", `search|${q.trim()}`) });
    if (selected && selected.loader !== "vanilla") out.push({ key: "search:modrinth", group: "Search", label: `Find “${q.trim()}” on Modrinth`, icon: ArrowRight, words: "", go: go("mods", `modrinth|${q.trim()}`) });
    return out;
  }, [q, targets, profiles, selected, mods, bedrock, setPage, selectProfile, onClose]);

  useEffect(() => setAt(0), [q]);
  useEffect(() => {
    list.current?.querySelector<HTMLElement>(`[data-i="${at}"]`)?.scrollIntoView({ block: "nearest" });
  }, [at]);

  const key = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") { e.preventDefault(); setAt((a) => Math.min(items.length - 1, a + 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setAt((a) => Math.max(0, a - 1)); }
    else if (e.key === "Enter") { e.preventDefault(); items[at]?.go(); }
    else if (e.key === "Escape") { e.preventDefault(); onClose(); }
  };

  let last = "";
  return (
    <div className="modal-bg palette-bg" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="palette" role="dialog" aria-modal="true" aria-label="Search">
        <div className="palette-input">
          <Search size={18} />
          <input autoFocus placeholder="Search pages, profiles, mods, Bedrock modules, settings…" value={q} onChange={(e) => setQ(e.target.value)} onKeyDown={key} aria-label="Search" />
          <kbd>Esc</kbd>
        </div>
        <div className="palette-list" ref={list} role="listbox">
          {items.length === 0 && <div className="palette-empty">Nothing found</div>}
          {items.map((it, n) => {
            const head = it.group !== last ? it.group : null;
            last = it.group;
            return (
              <div key={it.key}>
                {head && <div className="palette-group">{head}</div>}
                <div className={`palette-item ${n === at ? "on" : ""}`} data-i={n} role="option" aria-selected={n === at} onMouseMove={() => setAt(n)} onClick={it.go}>
                  <span className="palette-icon"><it.icon size={16} /></span>
                  <span className="grow ellipsis">{it.label}{it.hint && <span className="palette-hint"> · {it.hint}</span>}</span>
                  {n === at && <CornerDownLeft size={14} className="faint" />}
                </div>
              </div>
            );
          })}
        </div>
        <div className="palette-foot"><span><kbd>↑</kbd><kbd>↓</kbd> move</span><span><kbd>Enter</kbd> open</span><span><kbd>Ctrl</kbd><kbd>K</kbd> anytime</span></div>
      </div>
    </div>
  );
}
