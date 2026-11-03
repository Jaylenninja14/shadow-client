import { useEffect, useState } from "react";
import { api, type LoaderKind, type ProfileKind, type SetupReport, type VersionSummary } from "../api";
import { presetLabel } from "../format";
import { useStore } from "../store";
import { Alert, Button, Field, Modal, Select, Spinner, Toggle } from "./ui";

const KINDS: { value: ProfileKind; label: string; loader: LoaderKind; preset: string }[] = [
  { value: "performance", label: "Performance (Fabric + Sodium)", loader: "fabric", preset: "auto" },
  { value: "pvp", label: "PvP (Fabric + Sodium, High FPS)", loader: "fabric", preset: "high" },
  { value: "vanilla", label: "Vanilla", loader: "vanilla", preset: "balanced" },
  { value: "survival", label: "Survival", loader: "fabric", preset: "balanced" },
  { value: "modded", label: "Modded", loader: "fabric", preset: "balanced" },
  { value: "custom", label: "Custom", loader: "fabric", preset: "balanced" },
];

export function NewProfileDialog({ onClose, initialVersion, initialKind }: { onClose: () => void; initialVersion?: string; initialKind?: ProfileKind }) {
  const { settings, appState, refreshProfiles, run } = useStore();
  const [kind, setKind] = useState<ProfileKind>(initialKind ?? "performance");
  const [name, setName] = useState("");
  const [versions, setVersions] = useState<VersionSummary[]>([]);
  const [snapshots, setSnapshots] = useState(settings?.showSnapshots ?? false);
  const [mc, setMc] = useState(initialVersion ?? settings?.defaultVersion ?? "");
  const [loader, setLoader] = useState<LoaderKind>("fabric");
  const [loaderOk, setLoaderOk] = useState<boolean | null>(null);
  const [preset, setPreset] = useState<string>(settings?.defaultPreset ?? "auto");
  const [features, setFeatures] = useState(initialKind === "pvp");
  const [busy, setBusy] = useState(false);
  const [report, setReport] = useState<SetupReport | null>(null);

  useEffect(() => {
    api.listVersions(snapshots, settings?.showOldVersions ?? false).then((v) => {
      setVersions(v.versions);
      if (!mc) setMc(v.latestRelease);
    }).catch(() => undefined);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [snapshots]);

  useEffect(() => {
    if (!initialVersion && !settings?.defaultVersion && (kind === "performance" || kind === "pvp")) {
      api.recommendedPerformanceVersion().then(setMc).catch(() => undefined);
    }
    const k = KINDS.find((x) => x.value === kind)!;
    setLoader(k.loader === "fabric" && settings?.defaultLoader === "quilt" ? "quilt" : k.loader);
    setPreset(kind === "pvp" ? "high" : settings?.defaultPreset ?? k.preset);
    setFeatures(kind === "pvp");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [kind]);

  useEffect(() => {
    if (loader === "vanilla" || !mc) {
      setLoaderOk(true);
      return;
    }
    setLoaderOk(null);
    api.loaderGameVersions(loader).then((l) => setLoaderOk(l.some((x) => x.version === mc))).catch(() => setLoaderOk(null));
  }, [loader, mc]);

  const effectivePreset = preset === "auto" ? appState?.benchmark?.recommendedPreset ?? "balanced" : preset;

  if (report) {
    return (
      <Modal title="Profile ready" onClose={onClose} footer={<Button variant="primary" onClick={onClose}>Done</Button>}>
        {report.installed.length > 0 && <Alert tone="success" title="Installed and verified">{report.installed.join("\n")}</Alert>}
        {report.notes.map((n, i) => <div className="mt" key={i}><Alert tone={n.includes("not") || n.includes("skipped") ? "warning" : "info"}>{n}</Alert></div>)}
        {report.installed.length === 0 && report.notes.length === 0 && <p className="dim">The profile was created. Game files download on first launch.</p>}
      </Modal>
    );
  }

  return (
    <Modal
      title="New profile"
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button
            variant="primary"
            disabled={busy || !mc || !name.trim() || loaderOk === false}
            onClick={async () => {
              setBusy(true);
              const r = await run(() => api.createProfile({ name: name.trim(), kind, mcVersion: mc, loader, loaderVersion: null, gameDir: null, preset: effectivePreset, builtinMods: loader !== "vanilla", ingameFeatures: loader !== "vanilla" && features }));
              setBusy(false);
              if (r) {
                await refreshProfiles();
                setReport(r.report);
              }
            }}
          >
            {busy ? "Setting up…" : "Create profile"}
          </Button>
        </>
      }
    >
      <div className="form-grid">
        <Field label="Profile type" full>
          <Select value={kind} onChange={setKind} options={KINDS.map((k) => ({ value: k.value, label: k.label }))} />
        </Field>
        <Field label="Name" full>
          <input className="input" value={name} maxLength={48} placeholder={kind === "pvp" ? "PvP" : "My profile"} onChange={(e) => setName(e.target.value)} autoFocus />
        </Field>
        <Field label="Minecraft version" hint={<label className="row"><Toggle on={snapshots} onChange={setSnapshots} label="Show snapshots" /> Show snapshots</label>}>
          <select className="select" value={mc} onChange={(e) => setMc(e.target.value)}>
            {versions.length === 0 && <option value={mc}>{mc || "Loading…"}</option>}
            {versions.map((v) => (
              <option key={v.id} value={v.id}>{v.id}{v.kind !== "release" ? ` (${v.kind})` : ""}{v.installed ? " ✓" : ""}</option>
            ))}
          </select>
        </Field>
        <Field label="Loader" hint={loaderOk === false ? `${loader} does not support ${mc} yet` : loaderOk === null && loader !== "vanilla" ? "Checking support…" : undefined}>
          <Select value={loader} onChange={setLoader} options={[{ value: "fabric", label: "Fabric" }, { value: "quilt", label: "Quilt" }, { value: "vanilla", label: "Vanilla (no mods)" }]} />
        </Field>
        <Field label="Graphics preset" hint={preset === "auto" ? `Uses ${presetLabel(effectivePreset)}${appState?.benchmark ? " (from your benchmark)" : " (run the benchmark on the Performance page for a tailored pick)"}` : "Applied the first time the profile starts"}>
          <Select value={preset} onChange={setPreset} options={[{ value: "auto", label: "Automatic" }, { value: "low", label: "Low-end" }, { value: "balanced", label: "Balanced" }, { value: "high", label: "High FPS (PvP)" }, { value: "custom", label: "Custom" }]} />
        </Field>
        {loader !== "vanilla" && (
          <Field label="Built-in mods" hint="Always included and always on when a compatible build exists for this version.">
            <div className="small dim">Marlow's Crystal Optimizer · Always Shield</div>
          </Field>
        )}
        {loader !== "vanilla" && (
          <Field label="Lunar-style PvP HUD" hint="Shadow Client's own mod menu (HUD, zoom, hit color, motion blur, chat heads and more) is always built in. This switches on the usual PvP HUD to start with: FPS, CPS, keystrokes, ping, armor and potions.">
            <label className="row"><Toggle on={features} onChange={setFeatures} label="PvP HUD" /> Start with the PvP HUD</label>
          </Field>
        )}
      </div>
      {(kind === "performance" || kind === "pvp") && loader !== "vanilla" && (
        <div className="mt">
          <Alert tone="info" title="Automatic performance setup">
            {settings?.autoInstallSodium
              ? `Sodium plus the compatible performance mods from Settings (${settings.performanceMods.join(", ")}) are resolved for Minecraft ${mc || "…"}; anything without a matching build is skipped and explained.`
              : "Automatic Sodium install is turned off in Settings → Performance."}
          </Alert>
        </div>
      )}
      {busy && <div className="row mt dim"><Spinner /> Resolving loader and mods, downloading and verifying…</div>}
    </Modal>
  );
}
