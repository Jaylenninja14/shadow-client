import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Blocks, Boxes, Camera, Download, Gauge, Layers, Paintbrush, Pickaxe, Search, Server, Settings as SettingsIcon, Sparkles, Tags, UserRound, Users } from "lucide-react";
import { api } from "./api";
import { branding } from "./branding";
import { AccountDialog } from "./components/AccountDialog";
import { CommandPalette, type PaletteTarget } from "./components/CommandPalette";
import { Onboarding } from "./components/Onboarding";
import { ShareImportDialog } from "./components/ShareDialogs";
import { UpdateNotice } from "./components/UpdateNotice";
import { Badge, useSlidingPill } from "./components/ui";
import { loaderLabel } from "./format";
import { Bedrock } from "./pages/Bedrock";
import { Downloads } from "./pages/Downloads";
import { Features } from "./pages/Features";
import { Home } from "./pages/Home";
import { Modpacks } from "./pages/Modpacks";
import { Mods } from "./pages/Mods";
import { Performance } from "./pages/Performance";
import { Profiles } from "./pages/Profiles";
import { Screenshots } from "./pages/Screenshots";
import { Servers } from "./pages/Servers";
import { Settings } from "./pages/Settings";
import { Skins } from "./pages/Skins";
import { Versions } from "./pages/Versions";
import { useStore, type Page } from "./store";

/** The two games, as big tiles at the top of the sidebar. */
const GAMES: PaletteTarget[] = [
  { id: "home", label: "Java Edition", icon: Pickaxe, keywords: "home play java launch" },
  { id: "bedrock", label: "Bedrock Edition", icon: Blocks, keywords: "bedrock mod menu worlds packs windows" },
];

const NAV: { group: string; items: PaletteTarget[] }[] = [
  { group: "Play", items: [
    { id: "profiles", label: "Profiles", icon: Users, keywords: "modpack setup new copy share" },
    { id: "servers", label: "Servers", icon: Server, keywords: "multiplayer join ip" },
    { id: "versions", label: "Versions", icon: Tags, keywords: "minecraft release snapshot install" },
  ] },
  { group: "Customize", items: [
    { id: "mods", label: "Mods", icon: Boxes, keywords: "modrinth curseforge install update shaders resource packs" },
    { id: "features", label: "Features", icon: Sparkles, keywords: "hud mod menu hotkeys pvp in-game" },
    { id: "modpacks", label: "Modpacks", icon: Layers, keywords: "mrpack" },
    { id: "skins", label: "Skins", icon: Paintbrush, keywords: "skin editor cape" },
    { id: "screenshots", label: "Screenshots", icon: Camera, keywords: "pictures images" },
  ] },
  { group: "Launcher", items: [
    { id: "performance", label: "Performance", icon: Gauge, keywords: "fps boost benchmark graphics preset" },
    { id: "downloads", label: "Downloads", icon: Download, keywords: "progress queue" },
    { id: "settings", label: "Settings", icon: SettingsIcon, keywords: "options preferences theme" },
  ] },
];
const TARGETS: PaletteTarget[] = [...GAMES, ...NAV.flatMap((g) => g.items)];

function PageView({ page }: { page: Page }) {
  switch (page) {
    case "home": return <Home />;
    case "mods": return <Mods />;
    case "features": return <Features />;
    case "modpacks": return <Modpacks />;
    case "versions": return <Versions />;
    case "performance": return <Performance />;
    case "skins": return <Skins />;
    case "bedrock": return <Bedrock />;
    case "servers": return <Servers />;
    case "screenshots": return <Screenshots />;
    case "profiles": return <Profiles />;
    case "settings": return <Settings />;
    case "downloads": return <Downloads />;
  }
}

export default function App() {
  const { page, setPage, subPage, jobs, profiles, selected, selectProfile, activeAccount, appState, info, toasts, dismissToast, settings, games, toast } = useStore();
  const [accounts, setAccounts] = useState(false);
  const [palette, setPalette] = useState(false);
  const nav = useRef<HTMLElement>(null);
  const navPill = useSlidingPill(nav, ".nav-item.active", [page]);

  // Ctrl+K (or Ctrl+F outside a text box) opens search from anywhere
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = e.target instanceof HTMLElement && (e.target.closest("input, textarea, select, [contenteditable]") !== null);
      if (e.ctrlKey && (e.key === "k" || (e.key === "f" && !typing))) {
        e.preventDefault();
        setPalette((v) => !v);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  const [onboarding, setOnboarding] = useState(false);
  const [welcomeClosed, setWelcomeClosed] = useState(false);
  // a shadowclient:// share link Windows opened us with (or one clicked while we're open)
  const [shareLink, setShareLink] = useState<string | null>(null);
  const activeJobs = jobs.filter((j) => j.status === "running").length;

  useEffect(() => {
    if (appState && !appState.onboardingDone) setOnboarding(true);
  }, [appState?.onboardingDone]);

  useEffect(() => {
    api.sharePending().then((l) => l && setShareLink(l)).catch(() => undefined);
    const off = listen<string>("share-link", (e) => {
      setShareLink(e.payload);
      api.sharePending().catch(() => undefined); // taken now
    });
    return () => { off.then((f) => f()); };
  }, []);

  // Optional startup update check (one request, user-enabled).
  useEffect(() => {
    if (settings?.checkModUpdatesOnStartup && selected && selected.loader !== "vanilla") {
      api.modsCheckUpdates(selected.id).then((m) => {
        const n = m.filter((x) => x.update).length;
        if (n > 0) toast("info", `${n} mod update${n === 1 ? "" : "s"} available`, `For ${selected.name}. Open Mods → Updates to review them.`);
      }).catch(() => undefined);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings?.checkModUpdatesOnStartup]);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img src={branding.logoUrl} alt="" />
          <div>
            <div className="brand-name">{branding.productName}</div>
            <div className="brand-sub">v{info?.version ?? ""}{info?.portable ? " · portable" : ""}</div>
          </div>
        </div>
        <div className="games" role="group" aria-label="Game">
          <button className={`game java ${page === "home" ? "active" : ""}`} onClick={() => setPage("home")}>
            <span className="game-art"><Pickaxe size={18} /></span>
            <span className="game-text">
              <b>Java Edition</b>
              <small className="ellipsis">{selected ? `${selected.name} · ${selected.mcVersion}` : "Pick a profile"}</small>
            </span>
            {selected && games[selected.id]?.state === "running" && <span className="game-live" title="Running" />}
          </button>
          <button className={`game bedrock ${page === "bedrock" ? "active" : ""}`} onClick={() => setPage("bedrock")}>
            <span className="game-art"><Blocks size={18} /></span>
            <span className="game-text">
              <b>Bedrock Edition</b>
              <small className="ellipsis">Mod menu · worlds · packs</small>
            </span>
          </button>
        </div>
        <nav className="nav" ref={nav}>
          {navPill && <span className="nav-pill" aria-hidden style={navPill} />}
          {NAV.map((g) => (
            <div key={g.group}>
              <div className="nav-group">{g.group}</div>
              {g.items.map((n) => (
                <div key={n.id} className={`nav-item ${page === n.id ? "active" : ""}`} onClick={() => setPage(n.id)} role="button" tabIndex={0} onKeyDown={(e) => e.key === "Enter" && setPage(n.id)}>
                  <n.icon size={17} />
                  {n.label}
                  {n.id === "downloads" && activeJobs > 0 && <span className="nav-badge">{activeJobs}</span>}
                </div>
              ))}
            </div>
          ))}
        </nav>
        <div className="sidebar-foot">Not affiliated with Mojang or Microsoft.</div>
      </aside>
      <main className="main">
        <div className="topbar">
          <span className="topbar-label"><Layers size={14} /> Profile</span>
          <select className="select" style={{ maxWidth: 280 }} value={selected?.id ?? ""} onChange={(e) => selectProfile(e.target.value)}>
            {profiles.length === 0 && <option value="">No profiles</option>}
            {profiles.map((p) => <option key={p.id} value={p.id}>{p.name} — {p.mcVersion} {loaderLabel(p.loader)}</option>)}
          </select>
          {selected && games[selected.id]?.state === "running" && <Badge tone="accent">Running</Badge>}
          <button className="search-btn" onClick={() => setPalette(true)} title="Search (Ctrl+K)">
            <Search size={15} />
            <span className="grow ellipsis">Search everything</span>
            <kbd>Ctrl K</kbd>
          </button>
          <span className="spacer" />
          <button className="account-chip" onClick={() => setAccounts(true)}>
            <span className="avatar">{activeAccount ? (activeAccount.username.match(/[a-z0-9]/i)?.[0] ?? "?").toUpperCase() : <UserRound size={15} />}</span>
            <span className="ellipsis">{activeAccount ? activeAccount.username : "Add account"}</span>
            {activeAccount?.kind === "offline" && <Badge tone="warning">Offline</Badge>}
          </button>
        </div>
        <UpdateNotice holdDialog={!appState || onboarding || (!appState.onboardingDone && !welcomeClosed)} />
        {/* keyed so opening the same page from search (a new tab or filter) starts it fresh */}
        <PageView key={`${page}:${subPage ?? ""}`} page={page} />
      </main>
      {palette && <CommandPalette targets={TARGETS} onClose={() => setPalette(false)} />}
      {accounts && <AccountDialog onClose={() => setAccounts(false)} />}
      {onboarding && <Onboarding onClose={() => { setOnboarding(false); setWelcomeClosed(true); }} />}
      {shareLink && !onboarding && <ShareImportDialog initialLink={shareLink} onClose={() => setShareLink(null)} />}
      <div className="toasts" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.level}`} onClick={() => dismissToast(t.id)}>
            <div className="t">{t.title}</div>
            {t.message && <div className="m">{t.message}</div>}
          </div>
        ))}
      </div>
    </div>
  );
}
