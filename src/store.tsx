// Global app state. Event-driven: backend events update state; no background polling.
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type Account, type AppInfo, type AppStateT, type GameState, type JobInfo, type ProcessSample, type Profile, type Settings } from "./api";

export type Page = "home" | "mods" | "features" | "modpacks" | "versions" | "performance" | "skins" | "bedrock" | "servers" | "screenshots" | "profiles" | "settings" | "downloads";

export interface Toast {
  id: number;
  level: "info" | "success" | "warning" | "error";
  title: string;
  message?: string;
}

interface LaunchProgress {
  stage: string;
  message: string;
}

interface Store {
  info: AppInfo | null;
  settings: Settings | null;
  saveSettings: (s: Settings) => Promise<Settings>;
  appState: AppStateT | null;
  patchAppState: (p: Parameters<typeof api.setAppState>[0]) => Promise<void>;
  profiles: Profile[];
  refreshProfiles: () => Promise<Profile[]>;
  selected: Profile | null;
  selectProfile: (id: string) => Promise<void>;
  accounts: Account[];
  activeAccount: Account | null;
  refreshAccounts: () => Promise<void>;
  jobs: JobInfo[];
  refreshJobs: () => Promise<void>;
  games: Record<string, GameState>;
  stats: Record<string, ProcessSample>;
  launchProgress: Record<string, LaunchProgress>;
  page: Page;
  setPage: (p: Page, sub?: string) => void;
  subPage: string | null;
  toasts: Toast[];
  toast: (level: Toast["level"], title: string, message?: string) => void;
  dismissToast: (id: number) => void;
  /** Run an async action and surface errors as toasts. */
  run: <T>(fn: () => Promise<T>, okMessage?: string) => Promise<T | undefined>;
}

const Ctx = createContext<Store | null>(null);

export function useStore(): Store {
  const s = useContext(Ctx);
  if (!s) throw new Error("store missing");
  return s;
}

export function StoreProvider({ children }: { children: ReactNode }) {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [appState, setAppState] = useState<AppStateT | null>(null);
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [jobs, setJobs] = useState<JobInfo[]>([]);
  const [games, setGames] = useState<Record<string, GameState>>({});
  const [stats, setStats] = useState<Record<string, ProcessSample>>({});
  const [launchProgress, setLaunchProgress] = useState<Record<string, LaunchProgress>>({});
  const [page, setPageState] = useState<Page>("home");
  const [subPage, setSubPage] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastId = useRef(1);
  const notificationsOn = useRef(true);

  const toast = useCallback((level: Toast["level"], title: string, message?: string) => {
    if (!notificationsOn.current && level !== "error") return;
    const id = toastId.current++;
    setToasts((t) => [...t.slice(-4), { id, level, title, message }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), level === "error" ? 9000 : 5000);
  }, []);
  const dismissToast = useCallback((id: number) => setToasts((t) => t.filter((x) => x.id !== id)), []);

  const run = useCallback(
    async <T,>(fn: () => Promise<T>, okMessage?: string): Promise<T | undefined> => {
      try {
        const r = await fn();
        if (okMessage) toast("success", okMessage);
        return r;
      } catch (e) {
        toast("error", "Something went wrong", e instanceof Error ? e.message : String(e));
        return undefined;
      }
    },
    [toast],
  );

  const refreshProfiles = useCallback(async () => {
    const p = await api.listProfiles();
    setProfiles(p);
    return p;
  }, []);
  const refreshAccounts = useCallback(async () => {
    setAccounts(await api.listAccounts());
    setAppState(await api.getAppState());
  }, []);
  const refreshJobs = useCallback(async () => setJobs(await api.downloadsList()), []);

  const patchAppState = useCallback(async (p: Parameters<typeof api.setAppState>[0]) => {
    setAppState(await api.setAppState(p));
  }, []);

  const setPage = useCallback(
    (p: Page, sub?: string) => {
      setPageState(p);
      setSubPage(sub ?? null);
      api.setAppState({ lastPage: p }).then(setAppState).catch(() => undefined);
    },
    [],
  );

  const saveSettings = useCallback(async (s: Settings) => {
    const saved = await api.saveSettings(s);
    setSettings(saved);
    return saved;
  }, []);

  // Initial load.
  useEffect(() => {
    (async () => {
      try {
        const [i, s, a] = await Promise.all([api.appInfo(), api.getSettings(), api.getAppState()]);
        setInfo(i);
        setSettings(s);
        setAppState(a);
        if (s.startupPage === "last" && a.lastPage) setPageState(a.lastPage as Page);
        const [profs] = await Promise.all([refreshProfiles(), refreshAccounts(), refreshJobs()]);
        if (s.defaultProfile && profs.some((p) => p.id === s.defaultProfile) && a.selectedProfile !== s.defaultProfile) {
          setAppState(await api.setAppState({ selectedProfile: s.defaultProfile }));
        }
        const running = await api.runningGames();
        setGames(Object.fromEntries(running.map((r) => [r.profileId, { profileId: r.profileId, state: "running", pid: r.pid, message: null, exitCode: null, crashReport: null, logFile: r.logFile, startedAt: r.startedAt } as GameState])));
        api.appReady().catch(() => undefined);
      } catch (e) {
        toast("error", "Could not load launcher data", e instanceof Error ? e.message : String(e));
      }
    })();
  }, [refreshProfiles, refreshAccounts, refreshJobs, toast]);

  useEffect(() => {
    notificationsOn.current = settings?.notifications ?? true;
    if (!settings) return;
    const root = document.documentElement;
    root.dataset.theme = settings.theme;
    root.style.setProperty("--accent", settings.accent);
  }, [settings]);

  // Backend events.
  useEffect(() => {
    const unsubs: Promise<() => void>[] = [];
    unsubs.push(
      listen<JobInfo>("download:progress", (e) => {
        setJobs((list) => {
          const i = list.findIndex((j) => j.id === e.payload.id);
          if (i === -1) return [e.payload, ...list];
          const copy = list.slice();
          copy[i] = e.payload;
          return copy;
        });
        if (e.payload.status === "failed") toast("error", `Download failed: ${e.payload.title}`, e.payload.error ?? undefined);
      }),
    );
    unsubs.push(
      listen<GameState>("game:state", (e) => {
        setGames((g) => ({ ...g, [e.payload.profileId]: e.payload }));
        if (e.payload.state !== "running" && e.payload.state !== "starting") {
          setStats((s) => {
            const c = { ...s };
            delete c[e.payload.profileId];
            return c;
          });
          setLaunchProgress((p) => {
            const c = { ...p };
            delete c[e.payload.profileId];
            return c;
          });
          refreshProfiles().catch(() => undefined);
        }
        if (e.payload.state === "crashed") toast("error", "Minecraft crashed", e.payload.message ?? undefined);
        if (e.payload.state === "running") toast("success", "Minecraft is running", e.payload.message ?? undefined);
      }),
    );
    unsubs.push(listen<{ profileId: string; sample: ProcessSample }>("game:stats", (e) => setStats((s) => ({ ...s, [e.payload.profileId]: e.payload.sample }))));
    unsubs.push(listen<{ profileId: string; stage: string; message: string }>("launch:progress", (e) => setLaunchProgress((p) => ({ ...p, [e.payload.profileId]: { stage: e.payload.stage, message: e.payload.message } }))));
    unsubs.push(listen<{ level: Toast["level"]; title: string; message: string }>("notice", (e) => toast(e.payload.level, e.payload.title, e.payload.message)));
    return () => {
      unsubs.forEach((p) => p.then((f) => f()));
    };
  }, [toast, refreshProfiles]);

  const selected = useMemo(() => {
    const id = appState?.selectedProfile;
    return profiles.find((p) => p.id === id) ?? profiles[0] ?? null;
  }, [profiles, appState?.selectedProfile]);

  const selectProfile = useCallback(async (id: string) => {
    setAppState(await api.setAppState({ selectedProfile: id }));
  }, []);

  const activeAccount = useMemo(() => accounts.find((a) => a.id === appState?.activeAccount) ?? null, [accounts, appState?.activeAccount]);

  const value: Store = {
    info, settings, saveSettings, appState, patchAppState, profiles, refreshProfiles, selected, selectProfile,
    accounts, activeAccount, refreshAccounts, jobs, refreshJobs, games, stats, launchProgress, page, setPage, subPage,
    toasts, toast, dismissToast, run,
  };
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
