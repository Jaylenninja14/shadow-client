import { useEffect, useState } from "react";
import { Download } from "lucide-react";
import { api, type UpdateInfo } from "../api";
import { useStore } from "../store";
import { Button, Modal } from "./ui";

/** The release notes' bullet points as plain text (the body is Markdown). */
function highlights(notes: string | null): string[] {
  return (notes ?? "")
    .split(/\r?\n/)
    .filter((l) => l.startsWith("- "))
    .map((l) => l.slice(2).replace(/\*\*|`/g, "").trim())
    .filter(Boolean)
    .slice(0, 5);
}

/**
 * "Update needed": checks GitHub on start and every 15 minutes. When a newer Shadow Client is out it asks once
 * per session in a dialog, and a bar that can't be closed stays at the top until the launcher is updated.
 * `holdDialog`: another dialog (the welcome screen) is open; the question waits until it closes.
 */
export function UpdateNotice({ holdDialog }: { holdDialog?: boolean }) {
  const { games, toast } = useStore();
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [dialog, setDialog] = useState(false);
  const [asked, setAsked] = useState(false);
  const [installing, setInstalling] = useState(false);
  const gameRunning = Object.values(games).some((g) => g?.state === "running");

  useEffect(() => {
    const check = () => api.updateCheck().then(setUpdate).catch(() => undefined);
    check();
    const timer = window.setInterval(check, 15 * 60 * 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (update?.newer && !asked && !holdDialog) {
      setAsked(true);
      setDialog(true);
    }
  }, [update?.newer, asked, holdDialog]);

  if (!update?.newer) return null;

  async function updateNow() {
    if (!update) return;
    if (!update.canInstall) {
      if (update.url) await api.openUrl(update.url);
      return;
    }
    setInstalling(true);
    try {
      // Shadow Client closes, the installer runs and opens the new version.
      await api.updateInstall();
    } catch (e) {
      setInstalling(false);
      toast("error", "The update didn't install", e instanceof Error ? e.message : String(e));
    }
  }

  const label = installing ? "Downloading update…" : update.canInstall ? "Update now" : "Download update";
  const blocked = gameRunning && update.canInstall;
  const points = highlights(update.notes);

  return (
    <>
      <div className="update-bar needed">
        <Download size={16} />
        <strong>Update needed</strong>
        <span className="small">Shadow Client {update.latest} is out. You have {update.current}.</span>
        <span className="spacer" />
        <Button variant="primary" size="sm" disabled={installing || blocked} title={blocked ? "Close the game first" : undefined} onClick={updateNow}>
          {label}
        </Button>
      </div>
      {dialog && (
        <Modal
          title="Update needed"
          onClose={() => !installing && setDialog(false)}
          footer={
            <>
              <Button onClick={() => setDialog(false)} disabled={installing}>Later</Button>
              <Button variant="primary" disabled={installing || blocked} onClick={updateNow}>{label}</Button>
            </>
          }
        >
          <p>
            <strong>Shadow Client {update.latest}</strong> is out. You have {update.current}.
          </p>
          {points.length > 0 && (
            <>
              <p className="dim small">What's new:</p>
              <ul className="update-points">
                {points.map((p) => <li key={p}>{p}</li>)}
              </ul>
            </>
          )}
          <p className="dim small">
            {blocked
              ? "Close Minecraft first, then update."
              : update.canInstall
                ? "Shadow Client closes, installs the update and opens again. Your profiles, worlds and settings stay."
                : "This copy can't update itself (portable). Download update opens the Shadow Client website, where you can get the new version; your profiles, worlds and settings stay."}
          </p>
        </Modal>
      )}
    </>
  );
}
