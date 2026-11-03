import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ExternalLink, Trash2, UserRound } from "lucide-react";
import { api, type Account, type DeviceCode } from "../api";
import { useStore } from "../store";
import { Alert, Badge, Button, Field, Modal } from "./ui";

/** Microsoft sign-in (device code; the password is only ever typed on microsoft.com) and offline test profiles. */
export function AccountDialog({ onClose }: { onClose: () => void }) {
  const { accounts, appState, refreshAccounts, run, settings, setPage, selected, toast } = useStore();
  const [code, setCode] = useState<DeviceCode | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [offlineName, setOfflineName] = useState("");

  useEffect(() => {
    const un = listen<{ ok: boolean; account?: Account; error?: { message: string }; cancelled?: boolean }>("auth:result", async (e) => {
      setCode(null);
      if (e.payload.ok) {
        setStatus(`Signed in as ${e.payload.account?.username}`);
        await refreshAccounts();
      } else if (!e.payload.cancelled) {
        setStatus(e.payload.error?.message ?? "Sign-in failed");
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, [refreshAccounts]);

  const hasClientId = !!settings?.msaClientId;
  return (
    <Modal title="Accounts" onClose={() => { if (code) api.msaCancel(); onClose(); }}>
      <p className="dim mb">Shadow Client has no account of its own. Use a Microsoft account to play online, or an offline profile for local testing.</p>
      {accounts.length > 0 && (
        <div className="list mb">
          {accounts.map((a) => (
            <div className="list-row" key={a.id}>
              <div className="icon-box"><UserRound size={20} /></div>
              <div className="grow">
                <strong>{a.username}</strong>
                <div className="small faint">{a.kind === "msa" ? "Microsoft account" : "Offline — singleplayer/LAN testing only"}</div>
              </div>
              {a.kind === "offline" && <Badge tone="warning">Offline</Badge>}
              {appState?.activeAccount === a.id ? <Badge tone="accent">Active</Badge> : <Button size="sm" onClick={() => run(async () => { await api.setActiveAccount(a.id); await refreshAccounts(); })}>Use</Button>}
              <Button size="sm" variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={() => run(async () => { await api.removeAccount(a.id); await refreshAccounts(); })} />
            </div>
          ))}
        </div>
      )}

      <h3>Microsoft account</h3>
      {!hasClientId && (
        <Alert tone="info" title="Microsoft sign-in needs one setup step" action={<Button size="sm" onClick={() => { onClose(); setPage("settings", "accounts"); }}>Open settings</Button>}>
          Microsoft only lets approved applications request Minecraft tokens. Add your Azure application (client) ID under Settings → Accounts &amp; services (docs/AUTH_SETUP.md explains how). Until then, use the official launcher to play online.
        </Alert>
      )}
      {hasClientId && !code && (
        <div className="row mt">
          <Button variant="primary" onClick={() => run(async () => { setStatus(null); setCode(await api.msaStart()); })}>Sign in with Microsoft</Button>
        </div>
      )}
      {code && (
        <Alert tone="info" title="Finish signing in on Microsoft's website">
          Open <a onClick={() => api.openUrl(code.verificationUri)}>{code.verificationUri}</a> and enter this code:
          <div className="row mt">
            <code style={{ fontSize: 22, letterSpacing: 3 }}>{code.userCode}</code>
            <Button size="sm" onClick={() => navigator.clipboard.writeText(code.userCode)}>Copy</Button>
            <Button size="sm" icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(code.verificationUri))}>Open page</Button>
            <Button size="sm" variant="ghost" onClick={() => { api.msaCancel(); setCode(null); }}>Cancel</Button>
          </div>
          <div className="small faint mt">Waiting for you to finish… The code expires in {Math.round(code.expiresIn / 60)} minutes.</div>
        </Alert>
      )}
      {status && <div className="mt"><Alert tone={status.startsWith("Signed in") ? "success" : "error"}>{status}</Alert></div>}

      <div className="sep" />
      <h3>Offline profile (testing)</h3>
      <p className="small dim mt">Offline profiles are for local testing only: singleplayer and LAN. They cannot join online-mode servers or Realms and do not prove game ownership.</p>
      <div className="row mt">
        <Field label="Player name">
          <input className="input" value={offlineName} maxLength={16} placeholder="Steve" onChange={(e) => setOfflineName(e.target.value)} />
        </Field>
        <div style={{ alignSelf: "flex-end" }}>
          <Button disabled={offlineName.length < 3} onClick={() => run(async () => { await api.addOfflineAccount(offlineName); setOfflineName(""); await refreshAccounts(); }, "Offline profile added")}>Add offline profile</Button>
        </div>
      </div>
      <div className="sep" />
      <div className="row">
        <span className="small dim grow">Prefer the official launcher? It keeps its own sign-in; Shadow Client never reads or copies it.{selected && ["vanilla", "fabric", "quilt"].includes(selected.loader) ? (settings?.officialOnlyThis ? ` Opening it shows only ${selected.name} there, with its mods.` : ` Opening it adds ${selected.name} and its mods there.`) : ""}</span>
        <Button size="sm" onClick={async () => { const m = await run(() => api.openOfficialWithProfile(selected?.id)); if (m) toast("info", "Minecraft Launcher", m); }}>Open official launcher</Button>
      </div>
    </Modal>
  );
}
