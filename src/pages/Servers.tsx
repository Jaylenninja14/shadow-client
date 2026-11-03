import { useCallback, useEffect, useState } from "react";
import { Play, Plus, RefreshCw, Server, Trash2, Wifi, WifiOff } from "lucide-react";
import { api, type BedrockServer, type SavedServer, type ServerStatus } from "../api";
import { Alert, Badge, Button, Empty, Spinner, Tabs } from "../components/ui";
import { useStore } from "../store";

// Servers: Java servers saved in Shadow Client (live status, one-click join through Quick Play)
// and the servers in Bedrock's own list (minecraftpe/external_servers.txt).

type Tab = "java" | "bedrock";

function StatusLine({ status }: { status: ServerStatus | undefined }) {
  if (!status) return <span className="small faint">Checking…</span>;
  if (!status.online) return <span className="small" style={{ color: "var(--danger)" }}><WifiOff size={12} /> {status.error ?? "Offline"}</span>;
  return (
    <span className="small dim">
      <Wifi size={12} style={{ color: "var(--success)" }} /> {status.pingMs ?? "?"} ms · {status.playersOnline ?? "?"}/{status.playersMax ?? "?"} players · {status.version}
    </span>
  );
}

export function Servers() {
  const [tab, setTab] = useState<Tab>("java");
  return (
    <div className="page">
      <div className="page-head">
        <div>
          <h1>Servers</h1>
          <div className="sub">Live status and one-click join</div>
        </div>
      </div>
      <Tabs<Tab> value={tab} onChange={setTab} tabs={[{ id: "java", label: "Java" }, { id: "bedrock", label: "Bedrock" }]} />
      {tab === "java" ? <JavaServers /> : <BedrockServers />}
    </div>
  );
}

function JavaServers() {
  const { run, selected } = useStore();
  const [servers, setServers] = useState<SavedServer[] | null>(null);
  const [status, setStatus] = useState<Record<string, ServerStatus>>({});
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");

  const ping = useCallback((list: SavedServer[]) => {
    setStatus({});
    for (const s of list) api.serverPingJava(s.address).then((st) => setStatus((cur) => ({ ...cur, [s.id]: st })));
  }, []);

  useEffect(() => {
    api.serversList().then((l) => {
      setServers(l);
      ping(l);
    });
  }, [ping]);

  const save = async (list: SavedServer[]) => {
    const saved = await run(() => api.serversSave(list));
    if (saved) {
      setServers(saved);
      return saved;
    }
    return null;
  };

  const add = async () => {
    if (!address.trim() || !servers) return;
    const saved = await save([...servers, { id: "", name, address }]);
    if (saved) {
      setName("");
      setAddress("");
      const s = saved[saved.length - 1];
      api.serverPingJava(s.address).then((st) => setStatus((cur) => ({ ...cur, [s.id]: st })));
    }
  };

  const join = (s: SavedServer) => {
    if (!selected) return;
    run(() => api.playJoin(selected.id, s.address), `Starting ${selected.name} and joining ${s.name}`);
  };

  return (
    <div>
      <div className="card mb">
        <div className="row wrap">
          <input className="input" style={{ width: 200 }} placeholder="Name (optional)" value={name} onChange={(e) => setName(e.target.value)} />
          <input className="input grow" placeholder="Address, e.g. play.example.net or 1.2.3.4:25565" value={address} onChange={(e) => setAddress(e.target.value)} onKeyDown={(e) => e.key === "Enter" && add()} />
          <Button variant="primary" icon={<Plus size={14} />} disabled={!address.trim()} onClick={add}>Add server</Button>
          <Button icon={<RefreshCw size={14} />} onClick={() => servers && ping(servers)}>Refresh</Button>
        </div>
        <p className="small faint mt">
          Join starts {selected ? <strong>{selected.name}</strong> : "the selected profile"} and connects as soon as the game is up. Servers that only publish an SRV record need their real address and port.
        </p>
      </div>
      {!servers && <Spinner />}
      {servers && servers.length === 0 && <Empty title="No servers yet" icon={<Server size={28} />}>Add one above to see its status and join it in one click.</Empty>}
      {servers && servers.length > 0 && (
        <div className="list">
          {servers.map((s) => (
            <div key={s.id} className="list-row">
              {status[s.id]?.icon ? <img src={status[s.id]!.icon!} alt="" width={40} height={40} style={{ imageRendering: "pixelated", borderRadius: 6 }} /> : <div className="server-icon"><Server size={18} /></div>}
              <div className="grow" style={{ minWidth: 0 }}>
                <div className="row"><strong className="ellipsis">{s.name}</strong><span className="small faint">{s.address}</span></div>
                {status[s.id]?.motd && <div className="small dim ellipsis" style={{ whiteSpace: "pre-line" }}>{status[s.id]!.motd}</div>}
                <StatusLine status={status[s.id]} />
              </div>
              <Button variant="primary" icon={<Play size={14} />} disabled={!selected} onClick={() => join(s)}>Join</Button>
              <Button variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={() => save(servers.filter((x) => x.id !== s.id))} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function BedrockServers() {
  const { run } = useStore();
  const [servers, setServers] = useState<BedrockServer[] | null>(null);
  const [status, setStatus] = useState<Record<number, ServerStatus>>({});
  const [error, setError] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [port, setPort] = useState("19132");

  const load = useCallback(async () => {
    try {
      const l = await api.bedrockServers(null);
      setServers(l);
      setError(null);
      setStatus({});
      for (const s of l) api.serverPingBedrock(`${s.address}:${s.port}`).then((st) => setStatus((cur) => ({ ...cur, [s.index]: st })));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => { load(); }, [load]);

  const add = async () => {
    const l = await run(() => api.bedrockServerAdd(null, name || address, address, Number(port) || 19132), "Added to Bedrock's server list");
    if (l) {
      setName("");
      setAddress("");
      load();
    }
  };

  if (error) return <Alert tone="info">{error}</Alert>;
  return (
    <div>
      <div className="card mb">
        <div className="row wrap">
          <input className="input" style={{ width: 180 }} placeholder="Name" value={name} onChange={(e) => setName(e.target.value)} />
          <input className="input grow" placeholder="Address, e.g. play.example.net" value={address} onChange={(e) => setAddress(e.target.value)} />
          <input className="input" style={{ width: 90 }} placeholder="Port" value={port} onChange={(e) => setPort(e.target.value.replace(/\D/g, ""))} />
          <Button variant="primary" icon={<Plus size={14} />} disabled={!address.trim()} onClick={add}>Add</Button>
          <Button icon={<RefreshCw size={14} />} onClick={load}>Refresh</Button>
        </div>
        <p className="small faint mt">This is Bedrock's own server list (the Servers tab in Bedrock), for the account that played last. Close Bedrock before adding or removing; the old list is kept in backups.</p>
      </div>
      {!servers && <Spinner />}
      {servers && servers.length === 0 && <Empty title="No Bedrock servers" icon={<Server size={28} />} />}
      {servers && servers.length > 0 && (
        <div className="list">
          {servers.map((s) => (
            <div key={s.index} className="list-row">
              <div className="server-icon"><Server size={18} /></div>
              <div className="grow" style={{ minWidth: 0 }}>
                <div className="row"><strong className="ellipsis">{s.name}</strong><span className="small faint">{s.address}:{s.port}</span><Badge>Bedrock</Badge></div>
                {status[s.index]?.motd && <div className="small dim ellipsis" style={{ whiteSpace: "pre-line" }}>{status[s.index]!.motd}</div>}
                <StatusLine status={status[s.index]} />
              </div>
              <Button variant="primary" icon={<Play size={14} />} onClick={() => run(() => api.bedrockJoin(s.address, s.port), "Opening Bedrock")}>Join</Button>
              <Button variant="ghost" icon={<Trash2 size={14} />} title="Remove" onClick={async () => { if (await run(() => api.bedrockServerRemove(null, s.index), "Removed from Bedrock's list")) load(); }} />
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
