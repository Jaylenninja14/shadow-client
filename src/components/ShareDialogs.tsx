import { useEffect, useState } from "react";
import { Check, Copy, Link2 } from "lucide-react";
import { api, type PlanResult, type Profile, type ShareExport, type SharePack } from "../api";
import { loaderLabel } from "../format";
import { useStore } from "../store";
import { PlanDialog } from "./PlanDialog";
import { Alert, Badge, Button, Field, Modal, Spinner } from "./ui";

/** Share a profile: a link with its Modrinth mods, resource packs and shaders (nothing is uploaded). */
export function ShareDialog({ profile, onClose }: { profile: Profile; onClose: () => void }) {
  const [share, setShare] = useState<ShareExport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  useEffect(() => {
    api.shareExport(profile.id).then(setShare).catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, [profile.id]);
  const copy = async () => {
    if (!share) return;
    await navigator.clipboard.writeText(share.link);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 2000);
  };
  return (
    <Modal title={`Share ${profile.name}`} onClose={onClose} footer={<Button onClick={onClose}>Done</Button>}>
      {!share && !error && <div className="row dim"><Spinner /> Making the link…</div>}
      {error && <Alert tone="warning">{error}</Alert>}
      {share && (
        <>
          <p className="dim">Send this link to a friend. It opens a page on the Shadow Client website with an <strong>Open in Shadow Client</strong> button, which makes the same profile on their PC and downloads the mods from Modrinth and CurseForge.</p>
          <div className="row mt">
            <input className="input grow" readOnly value={share.link} onFocus={(e) => e.target.select()} aria-label="Share link" />
            <Button variant="primary" icon={copied ? <Check size={14} /> : <Copy size={14} />} onClick={copy}>{copied ? "Copied" : "Copy link"}</Button>
          </div>
          <p className="small faint mt">{share.included.length} item{share.included.length === 1 ? "" : "s"} included: {share.included.slice(0, 12).join(", ")}{share.included.length > 12 ? "…" : ""}</p>
          {share.skipped.length > 0 && (
            <Alert tone="info" title={`${share.skipped.length} left out`}>
              Only files from Modrinth or CurseForge can be shared (friends download them from there): {share.skipped.join(", ")}
            </Alert>
          )}
        </>
      )}
    </Modal>
  );
}

/** Open a share link: show what's in it, then make the profile and confirm the downloads. */
export function ShareImportDialog({ initialLink, onClose }: { initialLink?: string; onClose: () => void }) {
  const { run, refreshProfiles, selectProfile, toast } = useStore();
  const [link, setLink] = useState(initialLink ?? "");
  const [pack, setPack] = useState<SharePack | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const [busy, setBusy] = useState(false);

  const read = async (text: string) => {
    setError(null);
    setPack(null);
    if (!text.trim()) return;
    try {
      const p = await api.shareRead(text);
      setPack(p);
      setName(p.n);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  useEffect(() => { if (initialLink) read(initialLink); }, [initialLink]);

  const add = async () => {
    setBusy(true);
    const r = await run(() => api.shareImport(link, name));
    setBusy(false);
    if (!r) return;
    await refreshProfiles();
    await selectProfile(r.profile.id);
    setPlan(r.plan);
  };

  if (plan) {
    return (
      <PlanDialog
        result={plan}
        onClose={onClose}
        onDone={async () => { await refreshProfiles(); toast("success", "Pack added", `${name} is ready to play.`); onClose(); }}
      />
    );
  }

  const counts = pack ? { m: pack.i.filter((i) => i.k === "m").length, r: pack.i.filter((i) => i.k === "r").length, s: pack.i.filter((i) => i.k === "s").length } : null;
  return (
    <Modal
      title="Add a shared pack"
      onClose={onClose}
      footer={<>
        <Button onClick={onClose}>Cancel</Button>
        <Button variant="primary" disabled={!pack || busy || !name.trim()} onClick={add}>{busy ? "Setting up…" : "Add to Shadow Client"}</Button>
      </>}
    >
      {!initialLink && (
        <Field label="Share link" hint="Paste a Shadow Client share link someone sent you.">
          <div className="row">
            <input className="input grow" value={link} placeholder="https://…/share.html#…" onChange={(e) => setLink(e.target.value)} onBlur={() => read(link)} />
            <Button icon={<Link2 size={14} />} onClick={() => read(link)}>Read</Button>
          </div>
        </Field>
      )}
      {error && <Alert tone="warning">{error}</Alert>}
      {pack && counts && (
        <>
          <div className="row wrap mt" style={{ gap: 6 }}>
            <Badge tone="accent">Minecraft {pack.m}</Badge>
            <Badge>{loaderLabel(pack.l as Profile["loader"])}</Badge>
            <Badge>{counts.m} mods</Badge>
            {counts.r > 0 && <Badge>{counts.r} resource packs</Badge>}
            {counts.s > 0 && <Badge>{counts.s} shaders</Badge>}
          </div>
          <Field label="Profile name"><input className="input" value={name} maxLength={48} onChange={(e) => setName(e.target.value)} /></Field>
          <p className="small faint">{pack.i.slice(0, 20).map((i) => i.t).join(", ")}{pack.i.length > 20 ? "…" : ""}</p>
          <p className="small dim">A new profile is made and you see the full download list before anything is installed. Everything comes from Modrinth or CurseForge and is checked against the published hashes.</p>
        </>
      )}
    </Modal>
  );
}
