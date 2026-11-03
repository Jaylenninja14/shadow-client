import { useCallback, useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Brush, Copy, Download, Eraser, FileUp, Grid3x3, Layers, Paintbrush, PaintBucket, Pipette, Plus, Redo2, Trash2, Undo2, UploadCloud, UserRound, Wand2 } from "lucide-react";
import { api, type SkinWithData } from "../api";
import { Alert, Badge, Button, Confirm, Empty, Field, Modal, Select, Spinner } from "../components/ui";
import { relativeTime } from "../format";
import { drawPreview, faceRect, FACES, floodFill, getPx, hexToRgba, loadPixels, locate, parts, rgbaToHex, setPx, SIZE, template, toDataUrl, type Pixels, type SkinModel } from "../skin";
import { useStore } from "../store";

interface Draft {
  id: string | null;
  name: string;
  model: SkinModel;
  px: Pixels;
  source: string;
}

export function Skins() {
  const { run, activeAccount, profiles, selected } = useStore();
  const [list, setList] = useState<SkinWithData[] | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [playerOpen, setPlayerOpen] = useState(false);
  const [applyFor, setApplyFor] = useState<SkinWithData | null>(null);
  const [uploadFor, setUploadFor] = useState<SkinWithData | null>(null);
  const [deleteFor, setDeleteFor] = useState<SkinWithData | null>(null);
  const [message, setMessage] = useState<{ tone: "success" | "info" | "error"; text: string } | null>(null);

  const load = useCallback(async () => setList(await api.skinsList()), []);
  useEffect(() => { load(); }, [load]);

  const edit = async (s: SkinWithData, asCopy = false) => {
    const { px } = await loadPixels(s.dataUrl);
    setDraft({ id: asCopy ? null : s.id, name: asCopy ? `${s.name} copy`.slice(0, 40) : s.name, model: s.model, px, source: asCopy ? "created" : s.source });
  };

  const importFile = async () => {
    const path = await open({ multiple: false, filters: [{ name: "Skin (PNG)", extensions: ["png"] }] });
    if (typeof path !== "string") return;
    const dataUrl = await run(() => api.skinReadFile(path));
    if (!dataUrl) return;
    const { px, legacy } = await loadPixels(dataUrl);
    const base = path.split(/[\\/]/).pop()!.replace(/\.png$/i, "").slice(0, 40) || "Imported skin";
    setDraft({ id: null, name: base, model: "classic", px, source: "file" });
    if (legacy) setMessage({ tone: "info", text: "This was an old 64×32 skin; it was converted to the modern 64×64 layout (left arm and leg mirrored from the right)." });
  };

  return (
    <div className="page">
      <div className="page-head">
        <div><h1>Skins</h1><div className="sub">Create, edit and keep skins in your library, then use them in game.</div></div>
        <div className="actions">
          <Button icon={<UserRound size={14} />} onClick={() => setPlayerOpen(true)}>From player name</Button>
          <Button icon={<FileUp size={14} />} onClick={importFile}>Import PNG</Button>
          <Button variant="primary" icon={<Plus size={14} />} onClick={() => setDraft({ id: null, name: "My skin", model: "classic", px: template("classic"), source: "created" })}>New skin</Button>
        </div>
      </div>
      <Alert tone="info" title="Two ways to wear a skin">
        <strong>Upload to account</strong> changes your real Minecraft skin (everyone sees it online) — needs a signed-in Microsoft account.{"\n"}
        <strong>Use in a profile</strong> works offline too: it adds a small resource pack so <em>this PC</em> shows the skin in that profile. Other players won't see it.
      </Alert>
      {message && <div className="mt"><Alert tone={message.tone} action={<Button size="sm" variant="ghost" onClick={() => setMessage(null)}>Dismiss</Button>}>{message.text}</Alert></div>}
      <div className="mt" />
      {!list && <div className="row dim"><Spinner /> Loading…</div>}
      {list && list.length === 0 && (
        <Empty title="No skins yet" icon={<Paintbrush size={36} />}>
          <p className="dim">Start from the built-in template, import a PNG, or copy a player's public skin.</p>
        </Empty>
      )}
      <div className="grid auto">
        {list?.map((s) => (
          <div className="card" key={s.id}>
            <div className="row" style={{ alignItems: "flex-start", gap: 14 }}>
              <SkinThumb dataUrl={s.dataUrl} model={s.model} scale={4} />
              <div className="grow col" style={{ gap: 4, minWidth: 0 }}>
                <strong className="ellipsis">{s.name}</strong>
                <div className="row wrap"><Badge>{s.model === "slim" ? "Slim arms" : "Classic arms"}</Badge>{s.source.startsWith("player:") && <Badge tone="info">from {s.source.slice(7)}</Badge>}</div>
                <div className="small faint">Edited {relativeTime(s.updatedAt)}</div>
              </div>
            </div>
            <div className="row wrap mt">
              <Button size="sm" variant="primary" icon={<Brush size={14} />} onClick={() => edit(s)}>Edit</Button>
              <Button size="sm" icon={<Wand2 size={14} />} onClick={() => setApplyFor(s)}>Use in a profile</Button>
              <Button size="sm" icon={<UploadCloud size={14} />} onClick={() => setUploadFor(s)}>Upload to account</Button>
              <Button size="sm" icon={<Copy size={14} />} title="Duplicate" onClick={() => edit(s, true)} />
              <Button size="sm" icon={<Download size={14} />} title="Export PNG" onClick={async () => {
                const dest = await save({ defaultPath: `${s.name.replace(/[<>:"/\\|?*]/g, "_")}.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
                if (dest) await run(() => api.skinExport(s.id, dest), "Skin exported");
              }} />
              <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Delete" onClick={() => setDeleteFor(s)} />
            </div>
          </div>
        ))}
      </div>

      {draft && <SkinEditor draft={draft} onClose={() => setDraft(null)} onSaved={async () => { setDraft(null); await load(); }} />}
      {playerOpen && <PlayerImport onClose={() => setPlayerOpen(false)} onEdit={(d) => { setPlayerOpen(false); setDraft(d); }} />}
      {applyFor && (
        <ApplyDialog skin={applyFor} profiles={profiles.filter((p) => !p.external)} initial={selected?.id ?? ""} onClose={() => setApplyFor(null)} onDone={(text) => { setApplyFor(null); setMessage({ tone: "success", text }); }} />
      )}
      {uploadFor && (
        <Confirm title={`Upload “${uploadFor.name}” to your account?`} confirmLabel="Upload" onCancel={() => setUploadFor(null)} onConfirm={async () => {
          const r = await run(() => api.skinUpload(uploadFor.id));
          setUploadFor(null);
          if (r) setMessage({ tone: "success", text: r });
        }}>
          {activeAccount?.kind === "msa" ? (
            <p>This replaces the skin of <strong>{activeAccount.username}</strong> for everyone, using Mojang's official skin API ({uploadFor.model === "slim" ? "slim" : "classic"} arms).</p>
          ) : (
            <Alert tone="warning" title="Needs a Microsoft account">The active account is {activeAccount ? "an offline profile" : "not set"}. Sign in with Microsoft first (account menu, top right), or use “Use in a profile” to see the skin locally.</Alert>
          )}
        </Confirm>
      )}
      {deleteFor && (
        <Confirm title={`Delete “${deleteFor.name}”?`} confirmLabel="Delete" danger onCancel={() => setDeleteFor(null)} onConfirm={async () => { await run(() => api.skinDelete(deleteFor.id)); setDeleteFor(null); await load(); }}>
          The PNG is moved to the launcher's backups folder, so it can be recovered.
        </Confirm>
      )}
    </div>
  );
}

export function SkinThumb({ dataUrl, model, scale }: { dataUrl: string; model: SkinModel; scale: number }) {
  const front = useRef<HTMLCanvasElement>(null);
  const back = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    let alive = true;
    loadPixels(dataUrl).then(({ px }) => {
      if (!alive) return;
      for (const [ref, side] of [[front, "front"], [back, "back"]] as const) {
        const ctx = ref.current?.getContext("2d");
        if (ctx) drawPreview(ctx, px, model, side, scale);
      }
    });
    return () => { alive = false; };
  }, [dataUrl, model, scale]);
  return (
    <div className="row" style={{ gap: 6 }}>
      <canvas ref={front} width={16 * scale} height={32 * scale} />
      <canvas ref={back} width={16 * scale} height={32 * scale} />
    </div>
  );
}

type Tool = "pencil" | "eraser" | "fill" | "picker";
const SCALE = 8;
const PALETTE = ["#000000", "#ffffff", "#7f7f7f", "#d8a273", "#8d5524", "#5a3a22", "#ffb627", "#e0961a", "#e74c3c", "#3b4a6b", "#3a6ea5", "#2ecc71", "#9b59b6", "#f1c40f", "#1abc9c", "#2b2b30"];

function SkinEditor({ draft, onClose, onSaved }: { draft: Draft; onClose: () => void; onSaved: () => void }) {
  const { run } = useStore();
  const pxRef = useRef<Pixels>(new Uint8ClampedArray(draft.px));
  const undo = useRef<Pixels[]>([]);
  const redo = useRef<Pixels[]>([]);
  const [name, setName] = useState(draft.name);
  const [model, setModel] = useState<SkinModel>(draft.model);
  const [tool, setTool] = useState<Tool>("pencil");
  const [color, setColor] = useState("#ffb627");
  const [recent, setRecent] = useState<string[]>([]);
  const [showOverlay, setShowOverlay] = useState(true);
  const [showGrid, setShowGrid] = useState(true);
  const [hover, setHover] = useState<string>("");
  const [version, setVersion] = useState(0);
  const [dirty, setDirty] = useState(draft.id === null);
  const [askClose, setAskClose] = useState(false);
  const uv = useRef<HTMLCanvasElement>(null);
  const front = useRef<HTMLCanvasElement>(null);
  const back = useRef<HTMLCanvasElement>(null);
  const painting = useRef<{ last: [number, number] | null } | null>(null);

  const bump = () => { setVersion((v) => v + 1); setDirty(true); };

  // Render the UV map and previews whenever pixels or view options change.
  useEffect(() => {
    const ctx = uv.current?.getContext("2d");
    if (!ctx) return;
    const px = pxRef.current;
    const overlayRects = parts(model).filter((p) => p.overlay).flatMap((p) => FACES.map((f) => faceRect(p, f)));
    const inOverlay = (x: number, y: number) => overlayRects.some((r) => x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h);
    for (let y = 0; y < SIZE; y++) {
      for (let x = 0; x < SIZE; x++) {
        const [r, g, b, a] = getPx(px, x, y);
        const used = locate(model, x, y) !== null;
        ctx.fillStyle = !used ? "#15161b" : (x + y) % 2 === 0 ? "#2c2f38" : "#23262e";
        ctx.fillRect(x * SCALE, y * SCALE, SCALE, SCALE);
        if (a > 0) {
          const dim = !showOverlay && inOverlay(x, y);
          ctx.fillStyle = `rgba(${r},${g},${b},${(a / 255) * (dim ? 0.25 : 1)})`;
          ctx.fillRect(x * SCALE, y * SCALE, SCALE, SCALE);
        }
      }
    }
    if (showGrid) {
      ctx.strokeStyle = "rgba(255,255,255,0.06)";
      ctx.lineWidth = 1;
      for (let i = 0; i <= SIZE; i++) {
        ctx.beginPath(); ctx.moveTo(i * SCALE + 0.5, 0); ctx.lineTo(i * SCALE + 0.5, SIZE * SCALE); ctx.stroke();
        ctx.beginPath(); ctx.moveTo(0, i * SCALE + 0.5); ctx.lineTo(SIZE * SCALE, i * SCALE + 0.5); ctx.stroke();
      }
    }
    // Face outlines help find each body part.
    ctx.strokeStyle = "rgba(255,182,39,0.35)";
    for (const p of parts(model)) for (const f of FACES) {
      const r = faceRect(p, f);
      ctx.strokeRect(r.x * SCALE + 0.5, r.y * SCALE + 0.5, r.w * SCALE - 1, r.h * SCALE - 1);
    }
    for (const [ref, side] of [[front, "front"], [back, "back"]] as const) {
      const c = ref.current?.getContext("2d");
      if (c) drawPreview(c, px, model, side, 6, showOverlay);
    }
  }, [version, model, showOverlay, showGrid]);

  const snapshot = () => {
    undo.current.push(new Uint8ClampedArray(pxRef.current));
    if (undo.current.length > 60) undo.current.shift();
    redo.current = [];
  };
  const doUndo = () => {
    const prev = undo.current.pop();
    if (!prev) return;
    redo.current.push(new Uint8ClampedArray(pxRef.current));
    pxRef.current = prev;
    bump();
  };
  const doRedo = () => {
    const next = redo.current.pop();
    if (!next) return;
    undo.current.push(new Uint8ClampedArray(pxRef.current));
    pxRef.current = next;
    bump();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.tagName === "INPUT") return;
      if (e.ctrlKey && e.key.toLowerCase() === "z") { e.preventDefault(); doUndo(); }
      else if (e.ctrlKey && e.key.toLowerCase() === "y") { e.preventDefault(); doRedo(); }
      else if (e.key === "b") setTool("pencil");
      else if (e.key === "e") setTool("eraser");
      else if (e.key === "g") setTool("fill");
      else if (e.key === "i") setTool("picker");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const pixelAt = (e: React.PointerEvent<HTMLCanvasElement>): [number, number] => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = Math.floor(((e.clientX - rect.left) / rect.width) * SIZE);
    const y = Math.floor(((e.clientY - rect.top) / rect.height) * SIZE);
    return [Math.max(0, Math.min(SIZE - 1, x)), Math.max(0, Math.min(SIZE - 1, y))];
  };

  const useColor = (c: string) => {
    setColor(c);
    setRecent((r) => [c, ...r.filter((x) => x !== c)].slice(0, 8));
  };

  const paintAt = (x: number, y: number) => {
    if (!locate(model, x, y)) return; // unused texture area
    const c: [number, number, number, number] = tool === "eraser" ? [0, 0, 0, 0] : hexToRgba(color);
    setPx(pxRef.current, x, y, c);
  };

  const line = (a: [number, number], b: [number, number]) => {
    // Bresenham so fast strokes have no gaps
    let [x0, y0] = a;
    const [x1, y1] = b;
    const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0);
    const sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
    let err = dx + dy;
    for (;;) {
      paintAt(x0, y0);
      if (x0 === x1 && y0 === y1) break;
      const e2 = 2 * err;
      if (e2 >= dy) { err += dy; x0 += sx; }
      if (e2 <= dx) { err += dx; y0 += sy; }
    }
  };

  const onDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const [x, y] = pixelAt(e);
    if (tool === "picker") {
      const [r, g, b, a] = getPx(pxRef.current, x, y);
      if (a > 0) useColor(rgbaToHex(r, g, b));
      setTool("pencil");
      return;
    }
    snapshot();
    if (tool === "fill") {
      if (locate(model, x, y)) floodFill(pxRef.current, x, y, hexToRgba(color));
      bump();
      return;
    }
    e.currentTarget.setPointerCapture(e.pointerId);
    painting.current = { last: [x, y] };
    paintAt(x, y);
    if (tool === "pencil") useColor(color);
    bump();
  };
  const onMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const [x, y] = pixelAt(e);
    const loc = locate(model, x, y);
    setHover(loc ? `${loc.part.label} · ${loc.face} · (${x}, ${y})` : `(${x}, ${y}) unused`);
    if (!painting.current?.last) return;
    line(painting.current.last, [x, y]);
    painting.current.last = [x, y];
    bump();
  };
  const onUp = () => { painting.current = null; };

  const saveSkin = async () => {
    const r = await run(() => api.skinSave({ id: draft.id, name: name.trim() || "My skin", model, dataUrl: toDataUrl(pxRef.current), source: draft.source }), "Skin saved");
    if (r) onSaved();
  };

  const tools: { id: Tool; label: string; icon: typeof Brush; key: string }[] = [
    { id: "pencil", label: "Pencil", icon: Brush, key: "B" },
    { id: "eraser", label: "Eraser", icon: Eraser, key: "E" },
    { id: "fill", label: "Fill", icon: PaintBucket, key: "G" },
    { id: "picker", label: "Pick colour", icon: Pipette, key: "I" },
  ];

  return (
    <Modal wide title={draft.id ? `Edit skin` : "New skin"} onClose={() => (dirty ? setAskClose(true) : onClose())} footer={<>
      <span className="grow small faint">{hover || "Draw on the texture map; the preview updates live. Ctrl+Z / Ctrl+Y to undo/redo."}</span>
      <Button onClick={() => (dirty ? setAskClose(true) : onClose())}>Cancel</Button>
      <Button variant="primary" onClick={saveSkin}>Save to library</Button>
    </>}>
      <div className="row" style={{ alignItems: "flex-start", gap: 16, flexWrap: "wrap" }}>
        <canvas
          ref={uv}
          width={SIZE * SCALE}
          height={SIZE * SCALE}
          style={{ width: SIZE * SCALE, height: SIZE * SCALE, cursor: tool === "picker" ? "copy" : "crosshair", borderRadius: 6, border: "1px solid var(--border)", touchAction: "none" }}
          onPointerDown={onDown}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerLeave={() => setHover("")}
        />
        <div className="col grow" style={{ minWidth: 250, gap: 12 }}>
          <Field label="Name"><input className="input" value={name} maxLength={40} onChange={(e) => { setName(e.target.value); setDirty(true); }} /></Field>
          <Field label="Arm style"><Select value={model} onChange={(v) => { setModel(v); bump(); }} options={[{ value: "classic", label: "Classic (4 px arms)" }, { value: "slim", label: "Slim (3 px arms)" }]} /></Field>
          <div className="row wrap" style={{ gap: 6 }}>
            {tools.map((t) => (
              <button key={t.id} className={`btn sm ${tool === t.id ? "primary" : ""}`} title={`${t.label} (${t.key})`} onClick={() => setTool(t.id)}><t.icon size={14} /> {t.label}</button>
            ))}
          </div>
          <div className="row" style={{ gap: 8 }}>
            <input type="color" value={color} onChange={(e) => setColor(e.target.value)} style={{ width: 44, height: 34, border: "none", background: "none" }} />
            <input className="input mono" style={{ width: 100 }} value={color} onChange={(e) => /^#[0-9a-fA-F]{6}$/.test(e.target.value) && setColor(e.target.value)} />
            <Button size="sm" icon={<Undo2 size={14} />} title="Undo" onClick={doUndo} />
            <Button size="sm" icon={<Redo2 size={14} />} title="Redo" onClick={doRedo} />
          </div>
          <div className="row wrap" style={{ gap: 4 }}>
            {[...recent, ...PALETTE.filter((p) => !recent.includes(p))].slice(0, 20).map((c) => (
              <button key={c} title={c} onClick={() => useColor(c)} style={{ width: 22, height: 22, borderRadius: 4, border: c === color ? "2px solid var(--text)" : "1px solid var(--border-strong)", background: c, cursor: "pointer", padding: 0 }} />
            ))}
          </div>
          <div className="row wrap" style={{ gap: 6 }}>
            <button className={`btn sm ${showOverlay ? "primary" : ""}`} onClick={() => setShowOverlay((v) => !v)}><Layers size={14} /> Overlay layer</button>
            <button className={`btn sm ${showGrid ? "primary" : ""}`} onClick={() => setShowGrid((v) => !v)}><Grid3x3 size={14} /> Grid</button>
            <Button size="sm" onClick={() => { snapshot(); pxRef.current = template(model); bump(); }}>Reset to template</Button>
          </div>
          <div className="row" style={{ gap: 12, alignItems: "flex-end" }}>
            <div className="col" style={{ alignItems: "center", gap: 4 }}><canvas ref={front} width={16 * 6} height={32 * 6} /><span className="small faint">Front</span></div>
            <div className="col" style={{ alignItems: "center", gap: 4 }}><canvas ref={back} width={16 * 6} height={32 * 6} /><span className="small faint">Back</span></div>
          </div>
        </div>
      </div>
      {askClose && (
        <Confirm title="Discard changes?" confirmLabel="Discard" danger onCancel={() => setAskClose(false)} onConfirm={() => { setAskClose(false); onClose(); }}>
          Your edits to this skin haven't been saved.
        </Confirm>
      )}
    </Modal>
  );
}

function PlayerImport({ onClose, onEdit }: { onClose: () => void; onEdit: (d: Draft) => void }) {
  const [name, setName] = useState("");
  const [found, setFound] = useState<{ name: string; model: SkinModel; dataUrl: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  return (
    <Modal title="Copy a player's public skin" onClose={onClose} footer={<>
      <Button onClick={onClose}>Cancel</Button>
      <Button variant="primary" disabled={!found} onClick={async () => { if (!found) return; const { px } = await loadPixels(found.dataUrl); onEdit({ id: null, name: `${found.name}'s skin`.slice(0, 40), model: found.model, px, source: `player:${found.name}` }); }}>Open in editor</Button>
    </>}>
      <p className="dim mb">Looks the player up with Mojang's public profile API and downloads their current skin from textures.minecraft.net. Please respect other players' work.</p>
      <div className="row">
        <input className="input grow" value={name} maxLength={16} placeholder="Player name" onChange={(e) => setName(e.target.value)} onKeyDown={(e) => e.key === "Enter" && name.length >= 3 && document.getElementById("pc-find-skin")?.click()} />
        <button id="pc-find-skin" className="btn" disabled={busy || name.length < 3} onClick={async () => {
          setBusy(true); setError(null); setFound(null);
          try { setFound(await api.skinFetchPlayer(name)); } catch (e) { setError(e instanceof Error ? e.message : String(e)); } finally { setBusy(false); }
        }}>{busy ? <Spinner /> : null} Find</button>
      </div>
      {error && <div className="mt"><Alert tone="error">{error}</Alert></div>}
      {found && <div className="row mt"><SkinThumb dataUrl={found.dataUrl} model={found.model} scale={5} /><div><strong>{found.name}</strong><div className="small faint">{found.model === "slim" ? "Slim arms" : "Classic arms"}</div></div></div>}
    </Modal>
  );
}

function ApplyDialog({ skin, profiles, initial, onClose, onDone }: { skin: SkinWithData; profiles: { id: string; name: string; mcVersion: string; loader: string }[]; initial: string; onClose: () => void; onDone: (text: string) => void }) {
  const { run } = useStore();
  const [pid, setPid] = useState(profiles.some((p) => p.id === initial) ? initial : profiles[0]?.id ?? "");
  return (
    <Modal title={`Use “${skin.name}” in a profile`} onClose={onClose} footer={<>
      <Button onClick={onClose}>Cancel</Button>
      <Button variant="primary" disabled={!pid} onClick={async () => { const r = await run(() => api.skinApplyLocal(skin.id, pid)); if (r) onDone(r); }}>Apply</Button>
    </>}>
      {profiles.length === 0 ? <Alert tone="info">Create a profile first.</Alert> : (
        <>
          <Field label="Profile">
            <select className="select" value={pid} onChange={(e) => setPid(e.target.value)}>
              {profiles.map((p) => <option key={p.id} value={p.id}>{p.name} — {p.mcVersion}</option>)}
            </select>
          </Field>
          <p className="small dim mt">A resource pack called “Shadow Client Skin” is added to that profile and switched on (options.txt is backed up first). In game, players who use the default skins — including you with an offline profile — show this skin, only on this PC. Remove or turn it off under Mods → Resource packs.</p>
        </>
      )}
    </Modal>
  );
}
