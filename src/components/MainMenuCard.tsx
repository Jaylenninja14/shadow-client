import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { CheckCircle2, Image as ImageIcon } from "lucide-react";
import { api, type FeatureStatus } from "../api";
import { branding } from "../branding";
import shadowBackground from "../assets/shadow-background.jpg";
import { useStore } from "../store";
import { Badge, Button } from "./ui";

// Lunar-style title screen: the images rendered here are written into the profile together
// with a FancyMenu layout using the same geometry (see crates/pc-core/src/menu.rs).

const W = 1920;
const H = 1080;

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("Could not read that image"));
    img.src = src;
  });
}

/** Background: cover-fit, blurred and darkened like Lunar's menu. Shadow Client's own night scene if none chosen. */
async function renderBackground(src: string | null, blur: number, dim: number): Promise<HTMLCanvasElement> {
  const c = document.createElement("canvas");
  c.width = W;
  c.height = H;
  const ctx = c.getContext("2d")!;
  const image = await loadImage(src ?? shadowBackground).catch(() => null);
  if (image) {
    const img = image;
    const s = Math.max(W / img.width, H / img.height) * (blur > 0 ? 1.06 : 1); // overscan hides blurred edges
    const w = img.width * s;
    const h = img.height * s;
    ctx.filter = blur > 0 ? `blur(${blur * 2}px)` : "none";
    ctx.drawImage(img, (W - w) / 2, (H - h) / 2, w, h);
    ctx.filter = "none";
  } else {
    const g = ctx.createLinearGradient(0, 0, W, H);
    g.addColorStop(0, "#2a2140");
    g.addColorStop(0.55, "#3a2a6a");
    g.addColorStop(1, "#9b7bff");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, W, H);
  }
  ctx.fillStyle = `rgba(0,0,0,${dim / 100})`;
  ctx.fillRect(0, 0, W, H);
  return c;
}

async function renderLogo(): Promise<HTMLCanvasElement> {
  const c = document.createElement("canvas");
  c.width = 512;
  c.height = 320;
  const ctx = c.getContext("2d")!;
  const img = await loadImage(branding.logoUrl);
  ctx.shadowColor = "rgba(0,0,0,0.45)";
  ctx.shadowBlur = 12;
  ctx.drawImage(img, 156, 8, 200, 200);
  ctx.fillStyle = "#ffffff";
  ctx.font = "800 52px 'Segoe UI', system-ui, sans-serif";
  ctx.textAlign = "center";
  (ctx as CanvasRenderingContext2D & { letterSpacing: string }).letterSpacing = "6px";
  ctx.fillText(branding.productName.toUpperCase(), 256, 288);
  return c;
}

function renderButton(hover: boolean): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = 200;
  c.height = 20;
  const ctx = c.getContext("2d")!;
  ctx.fillStyle = hover ? "rgba(255,182,39,0.30)" : "rgba(18,18,24,0.58)";
  ctx.fillRect(1, 1, 198, 18);
  ctx.strokeStyle = hover ? "rgba(255,200,90,0.95)" : "rgba(255,255,255,0.32)";
  ctx.lineWidth = 1;
  ctx.strokeRect(0.5, 0.5, 199, 19);
  return c;
}

export function MainMenuCard({ profileId, feature, onInstall }: { profileId: string; feature: FeatureStatus | undefined; onInstall: (keys: string[]) => Promise<void> }) {
  const { run, toast } = useStore();
  const [src, setSrc] = useState<string | null>(null);
  // the Shadow background is drawn sharp; chosen photos get Lunar's blur and darkening
  const [blur, setBlur] = useState(0);
  const [dim, setDim] = useState(0);
  const [installed, setInstalled] = useState(false);
  const [busy, setBusy] = useState(false);
  const preview = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    api.menuState(profileId).then((s) => {
      setInstalled(s.installed);
      if (s.background) {
        setSrc(s.background);
        setBlur(0); // the saved background is already processed
        setDim(0);
      }
    }).catch(() => undefined);
  }, [profileId]);

  // Live preview of the in-game layout (same geometry as the FancyMenu layout file).
  useEffect(() => {
    let alive = true;
    (async () => {
      const [bg, logo] = await Promise.all([renderBackground(src, blur, dim), renderLogo()]);
      const c = preview.current;
      if (!alive || !c) return;
      const ctx = c.getContext("2d")!;
      const gs = c.width / 480; // preview of a 480×270 GUI-scaled screen
      ctx.drawImage(bg, 0, 0, c.width, c.height);
      const cx = c.width / 2;
      const cy = c.height / 2;
      ctx.drawImage(logo, cx - 60 * gs, cy - 112 * gs, 120 * gs, 75 * gs);
      const btn = renderButton(false);
      const button = (text: string, x: number, y: number, w: number) => {
        ctx.drawImage(btn, x, y, w, 20 * gs);
        ctx.fillStyle = "#fff";
        ctx.font = `${7 * gs}px 'Segoe UI', sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(text, x + w / 2, y + 10 * gs);
      };
      button("Singleplayer", cx - 100 * gs, cy - 24 * gs, 200 * gs);
      button("Multiplayer", cx - 100 * gs, cy + 2 * gs, 200 * gs);
      const by = c.height - 34 * gs;
      button("Mods", cx - 152 * gs, by, 98 * gs);
      button("Options...", cx - 49 * gs, by, 98 * gs);
      button("Quit Game", cx + 54 * gs, by, 98 * gs);
    })().catch((e) => toast("error", "Preview failed", String(e)));
    return () => {
      alive = false;
    };
  }, [src, blur, dim, toast]);

  const choose = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }] });
    if (typeof path !== "string") return;
    const data = await run(() => api.menuReadImage(path));
    if (data) {
      setSrc(data);
      setBlur(6);
      setDim(25);
    }
  };

  const apply = async () => {
    setBusy(true);
    try {
      const [bg, logo] = await Promise.all([renderBackground(src, blur, dim), renderLogo()]);
      const ok = await run(() =>
        api
          .menuApply(profileId, {
            background: bg.toDataURL("image/png"),
            logo: logo.toDataURL("image/png"),
            button: renderButton(false).toDataURL("image/png"),
            buttonHover: renderButton(true).toDataURL("image/png"),
          })
          .then(() => true),
      );
      if (ok) {
        setInstalled(true);
        if (feature?.installedFile) {
          toast("success", "Main menu saved", "It shows the next time this profile starts.");
        } else {
          toast("info", "Main menu saved", "One more step: confirm installing FancyMenu so the game can show it.");
          await onInstall(["custom-menu"]);
        }
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="card mt">
      <div className="card-head">
        <h3>Main menu</h3>
        <span className="small faint">Lunar-style title screen with your own background</span>
        <div className="actions">{installed && <Badge tone="accent"><CheckCircle2 size={12} /> Active in this profile</Badge>}</div>
      </div>
      <div className="row" style={{ alignItems: "flex-start", gap: 18, flexWrap: "wrap" }}>
        <canvas ref={preview} width={480} height={270} style={{ borderRadius: 8, border: "1px solid var(--border)", width: 480, maxWidth: "100%" }} />
        <div className="col grow" style={{ minWidth: 240, gap: 10 }}>
          <div className="row wrap">
            <Button icon={<ImageIcon size={14} />} onClick={choose}>{src ? "Change background…" : "Use my own image…"}</Button>
            {src && <Button variant="ghost" onClick={() => { setSrc(null); setBlur(0); setDim(0); }}>Shadow background</Button>}
          </div>
          <label className="small dim">
            Blur: {blur}
            <input type="range" min={0} max={20} value={blur} onChange={(e) => setBlur(Number(e.target.value))} />
          </label>
          <label className="small dim">
            Darken: {dim}%
            <input type="range" min={0} max={70} value={dim} onChange={(e) => setDim(Number(e.target.value))} />
          </label>
          <div className="row wrap">
            <Button variant="primary" disabled={busy || feature?.available === false} onClick={apply}>{busy ? "Saving…" : installed ? "Update menu" : "Apply to profile"}</Button>
            {installed && (
              <Button variant="ghost" onClick={() => run(async () => { await api.menuRemove(profileId); setInstalled(false); }, "Custom menu turned off")}>Turn off</Button>
            )}
          </div>
          {feature?.available === false && <div className="small" style={{ color: "var(--warning)" }}>{feature.checkNote}</div>}
          <p className="small faint">
            Shown in game by FancyMenu{feature?.installedFile ? " (installed)" : " — added when you apply"}. Buttons use Minecraft's font in game. Your image is copied into this profile's config/fancymenu folder; turning the menu off moves only these files to backups.
          </p>
        </div>
      </div>
    </div>
  );
}
