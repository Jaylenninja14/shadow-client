import { open } from "@tauri-apps/plugin-dialog";
import { Server } from "lucide-react";
import { api } from "../api";
import { useStore } from "../store";
import { Button } from "./ui";

// ShadowCombat is a Paper plugin: combat timing is decided by the server, so the rules go
// there and every client (Shadow Client included) follows them. Source: server-plugin/.

const COMMANDS: [string, string][] = [
  ["/shadowcombat", "show the current rules"],
  ["/shadowcombat hitdelay 0-10", "ticks before a target takes the next full hit (vanilla 10)"],
  ["/shadowcombat nocooldown on|off", "weapons always fully charged: full-damage hits, spear jabs and Lunge as fast as you click"],
  ["/shadowcombat noshielddelay on|off", "shields block instantly (vanilla 0.25 s)"],
  ["/shadowcombat enabled on|off", "master switch"],
  ["/combat on|off", "each player can turn the tweaks off for themselves"],
];

export function ServerPluginCard() {
  const { run, toast } = useStore();
  const install = async () => {
    const dir = await open({ directory: true, multiple: false, title: "Pick your Paper server's folder" });
    if (typeof dir !== "string") return;
    const path = await run(() => api.serverPluginInstall(dir));
    if (path) toast("success", "ShadowCombat installed", `${path} — restart the server to load it.`);
  };
  return (
    <div className="card mt">
      <div className="card-head">
        <h3>For your server: ShadowCombat</h3>
        <span className="small faint">Toggleable combat timing for a Paper 26.3 server you run</span>
        <div className="actions"><Button icon={<Server size={14} />} onClick={install}>Install on a server…</Button></div>
      </div>
      <p className="small dim mb">
        Hit delay, attack cooldown and shield delay are decided by the server, so these switches go on your server. Once they're on, every player's
        client follows them automatically. Ops change them in game; all are off by default except the master switch.
      </p>
      <table className="table small">
        <tbody>{COMMANDS.map(([c, d]) => <tr key={c}><td className="mono" style={{ whiteSpace: "nowrap" }}>{c}</td><td>{d}</td></tr>)}</tbody>
      </table>
    </div>
  );
}
