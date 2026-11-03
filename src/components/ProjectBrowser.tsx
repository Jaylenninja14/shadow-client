import { useCallback, useEffect, useState } from "react";
import { Download, ExternalLink, Search } from "lucide-react";
import { api, type CfMod, type ContentKind, type MrHit, type MrVersion, type PlanResult, type ProjectDetails } from "../api";
import { count, loaderLabel, relativeTime } from "../format";
import { useStore } from "../store";
import { PlanDialog } from "./PlanDialog";
import { Alert, Badge, Button, Empty, Img, Modal, Select, Spinner, Toggle } from "./ui";

const MR_CATEGORIES: { value: string; label: string; categories: string[]; keyword?: string }[] = [
  { value: "", label: "All categories", categories: [] },
  { value: "performance", label: "Performance", categories: ["optimization"] },
  { value: "pvp", label: "PvP (keyword)", categories: [], keyword: "pvp" },
  { value: "utility", label: "Utility / QoL", categories: ["utility"] },
  { value: "visual", label: "Visual", categories: ["decoration"] },
  { value: "library", label: "Libraries", categories: ["library"] },
];

const SORTS = [
  { value: "relevance", label: "Relevance" },
  { value: "downloads", label: "Downloads" },
  { value: "follows", label: "Popularity (follows)" },
  { value: "updated", label: "Recently updated" },
  { value: "newest", label: "Newest" },
];

export type BrowserKind = ContentKind | "modpack";

function cfClass(kind: BrowserKind): number {
  return kind === "mod" ? 6 : kind === "resourcepack" ? 12 : kind === "shader" ? 6552 : 4471;
}

/** Search Modrinth or CurseForge. Install goes through the resolver + PlanDialog. */
export function ProjectBrowser({ platform, kind, onInstallPack, onCreateProfile, initialQuery }: { platform: "modrinth" | "curseforge"; kind: BrowserKind; initialQuery?: string; onInstallPack?: (platform: "modrinth" | "curseforge", id: string, title: string) => void; onCreateProfile?: (mc: string) => void }) {
  const { selected, run, setPage } = useStore();
  const [query, setQuery] = useState(initialQuery ?? "");
  const [category, setCategory] = useState("");
  const [sort, setSort] = useState("relevance");
  const [onlyCompatible, setOnlyCompatible] = useState(kind !== "modpack");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [mrHits, setMrHits] = useState<MrHit[]>([]);
  const [cfHits, setCfHits] = useState<CfMod[]>([]);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [details, setDetails] = useState<{ id: string; title: string } | null>(null);
  const [plan, setPlan] = useState<PlanResult | null>(null);
  const [hasKey, setHasKey] = useState<boolean | null>(null);
  const limit = 20;

  useEffect(() => {
    if (platform === "curseforge") api.curseforgeStatus().then(setHasKey);
  }, [platform]);

  const search = useCallback(
    async (newOffset = 0) => {
      setLoading(true);
      setError(null);
      try {
        const cat = MR_CATEGORIES.find((c) => c.value === category);
        const compat = onlyCompatible && selected;
        const loader = compat && kind === "mod" ? selected!.loader : null;
        if (platform === "modrinth") {
          const r = await api.modrinthSearch({
            query: [query, cat?.keyword].filter(Boolean).join(" "),
            projectType: kind,
            gameVersion: compat ? selected!.mcVersion : null,
            loader: loader && loader !== "vanilla" ? loader : null,
            categories: kind === "mod" ? cat?.categories ?? [] : [],
            sort,
            offset: newOffset,
            limit,
            clientSideOnly: kind === "mod",
          });
          setMrHits(r.hits);
          setTotal(r.total_hits);
        } else {
          const r = await api.curseforgeSearch({
            query: [query, cat?.keyword].filter(Boolean).join(" "),
            classId: cfClass(kind),
            gameVersion: compat ? selected!.mcVersion : null,
            loader: loader && loader !== "vanilla" ? loader : null,
            categoryId: null,
            sort: sort === "follows" ? "popularity" : sort === "relevance" ? null : sort,
            offset: newOffset,
            limit,
          });
          setCfHits(r.data);
          setTotal(r.pagination.totalCount);
        }
        setOffset(newOffset);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      } finally {
        setLoading(false);
      }
    },
    [platform, kind, query, category, sort, onlyCompatible, selected],
  );

  useEffect(() => {
    if (platform === "curseforge" && hasKey !== true) return;
    const t = window.setTimeout(() => search(0), query ? 350 : 0);
    return () => window.clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, category, sort, onlyCompatible, selected?.id, platform, kind, hasKey]);

  const install = async (projectId: string, title: string, versionId: string | null = null) => {
    if (kind === "modpack") {
      onInstallPack?.(platform, projectId, title);
      return;
    }
    if (!selected) {
      setError("Create or select a profile first.");
      return;
    }
    const r = await run(() => api.planInstall({ profileId: selected.id, platform, projectId, versionId, kind }));
    if (r) setPlan(r);
  };

  if (platform === "curseforge" && hasKey === false) {
    return (
      <Alert tone="info" title="CurseForge needs an API key" action={<Button onClick={() => setPage("settings", "accounts")}>Add key</Button>}>
        CurseForge only allows API access with a personal key. Create one at console.curseforge.com and paste it under Settings → Accounts &amp; services. It is stored encrypted for your Windows account and never shared. See docs/CURSEFORGE_SETUP.md.
      </Alert>
    );
  }

  return (
    <div>
      <div className="row wrap mb">
        <div className="row grow" style={{ minWidth: 260 }}>
          <Search size={16} className="faint" />
          <input className="input grow" placeholder={`Search ${platform === "modrinth" ? "Modrinth" : "CurseForge"} by name, description or author…`} value={query} onChange={(e) => setQuery(e.target.value)} />
        </div>
        {kind === "mod" && <Select value={category} onChange={setCategory} options={MR_CATEGORIES.map((c) => ({ value: c.value, label: c.label }))} />}
        <Select value={sort} onChange={setSort} options={SORTS} />
        {selected && (
          <label className="row small dim">
            <Toggle on={onlyCompatible} onChange={setOnlyCompatible} label="Only compatible" />
            Only {selected.mcVersion}{kind === "mod" ? ` · ${loaderLabel(selected.loader)}` : ""}
          </label>
        )}
      </div>
      {error && <Alert tone="error" title="Search failed">{error}</Alert>}
      {loading && <div className="row dim"><Spinner /> Searching…</div>}
      {!loading && !error && (platform === "modrinth" ? mrHits.length : cfHits.length) === 0 && <Empty title="No results">Try another search term or turn off the compatibility filter.</Empty>}
      <div className="grid auto">
        {platform === "modrinth" &&
          mrHits.map((h) => (
            <div key={h.project_id} className="project-card" onClick={() => setDetails({ id: h.project_id, title: h.title })}>
              <Img src={h.icon_url} size={56} />
              <div className="grow col" style={{ gap: 4 }}>
                <div className="row">
                  <strong className="ellipsis grow">{h.title}</strong>
                </div>
                <div className="small faint">by {h.author ?? "unknown"} · {count(h.downloads)} downloads · updated {relativeTime(h.date_modified)}</div>
                <div className="desc">{h.description}</div>
                <div className="row wrap" style={{ gap: 4 }}>
                  {h.display_categories.slice(0, 4).map((c) => <Badge key={c}>{c}</Badge>)}
                </div>
              </div>
              <span onClick={(e) => e.stopPropagation()}><Button size="sm" variant="primary" icon={<Download size={14} />} onClick={() => install(h.project_id, h.title)}>Install</Button></span>
            </div>
          ))}
        {platform === "curseforge" &&
          cfHits.map((m) => (
            <div key={m.id} className="project-card" onClick={() => setDetails({ id: String(m.id), title: m.name })}>
              <Img src={m.logo?.thumbnailUrl} size={56} />
              <div className="grow col" style={{ gap: 4 }}>
                <strong className="ellipsis">{m.name}</strong>
                <div className="small faint">by {m.authors.map((a) => a.name).join(", ") || "unknown"} · {count(m.downloadCount)} downloads · updated {relativeTime(m.dateModified)}</div>
                <div className="desc">{m.summary}</div>
                <div className="row wrap" style={{ gap: 4 }}>
                  {m.categories.slice(0, 3).map((c) => <Badge key={c.id}>{c.name}</Badge>)}
                  {m.allowModDistribution === false && <Badge tone="warning">Manual download only</Badge>}
                </div>
              </div>
              <span onClick={(e) => e.stopPropagation()}><Button size="sm" variant="primary" icon={<Download size={14} />} onClick={() => install(String(m.id), m.name)}>Install</Button></span>
            </div>
          ))}
      </div>
      {total > limit && (
        <div className="row mt">
          <Button size="sm" disabled={offset === 0 || loading} onClick={() => search(Math.max(0, offset - limit))}>Previous</Button>
          <span className="small dim">{offset + 1}–{Math.min(offset + limit, total)} of {count(total)}</span>
          <Button size="sm" disabled={offset + limit >= total || loading} onClick={() => search(offset + limit)}>Next</Button>
        </div>
      )}
      {details && (
        <ProjectDetailsModal platform={platform} id={details.id} title={details.title} kind={kind} onClose={() => setDetails(null)} onInstall={(versionId) => { setDetails(null); install(details.id, details.title, versionId); }} />
      )}
      {plan && <PlanDialog result={plan} onClose={() => setPlan(null)} onDone={() => undefined} onCreateProfile={onCreateProfile} />}
    </div>
  );
}

function ProjectDetailsModal({ platform, id, title, kind, onClose, onInstall }: { platform: "modrinth" | "curseforge"; id: string; title: string; kind: BrowserKind; onClose: () => void; onInstall: (versionId: string | null) => void }) {
  const { selected, run } = useStore();
  const [mr, setMr] = useState<ProjectDetails | null>(null);
  const [versions, setVersions] = useState<MrVersion[] | null>(null);
  const [cf, setCf] = useState<CfMod | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    (async () => {
      try {
        if (platform === "modrinth") {
          const d = await api.modrinthProject(id);
          setMr(d);
          setVersions(await api.modrinthVersions(id, kind === "modpack" ? null : selected?.id ?? null));
        } else {
          setCf(await api.curseforgeMod(Number(id)));
        }
      } catch (e) {
        setErr(e instanceof Error ? e.message : String(e));
      }
    })();
  }, [platform, id, kind, selected?.id]);

  const url = platform === "modrinth" ? mr?.url : cf?.links?.websiteUrl;
  return (
    <Modal
      wide
      title={title}
      onClose={onClose}
      footer={
        <>
          {url && <Button icon={<ExternalLink size={14} />} onClick={() => run(() => api.openUrl(url))}>Open project page</Button>}
          <span className="grow" />
          <Button variant="primary" onClick={() => onInstall(null)}>Install latest compatible</Button>
        </>
      }
    >
      {err && <Alert tone="error">{err}</Alert>}
      {!mr && !cf && !err && <div className="row dim"><Spinner /> Loading…</div>}
      {mr && (
        <div className="col" style={{ gap: 12 }}>
          <div className="row" style={{ alignItems: "flex-start" }}>
            <Img src={mr.project.icon_url} size={72} />
            <div className="grow">
              <p>{mr.project.description}</p>
              <div className="kv mt">
                <span>Authors</span><span>{mr.authors.join(", ") || "—"}</span>
                <span>License</span><span>{mr.project.license?.name || mr.project.license?.id || "Not stated"}</span>
                <span>Downloads</span><span>{count(mr.project.downloads)}</span>
                <span>Loaders</span><span>{mr.project.loaders.join(", ") || "—"}</span>
                <span>Minecraft</span><span>{mr.project.game_versions.slice(-8).join(", ")}{mr.project.game_versions.length > 8 ? " …" : ""}</span>
                <span>Environment</span><span>client {mr.project.client_side ?? "?"} · server {mr.project.server_side ?? "?"}</span>
                <span>Updated</span><span>{relativeTime(mr.project.updated)}</span>
                <span>Source</span><span>Modrinth{mr.project.source_url ? " · source code available" : ""}</span>
              </div>
            </div>
          </div>
          {mr.project.client_side === "unsupported" && kind === "mod" && <Alert tone="warning" title="Server-side mod">Modrinth lists this as a server-side mod. In a client it only works in singleplayer/LAN worlds you host.</Alert>}
          <h3>{kind === "modpack" ? "Versions" : selected ? `Versions for ${selected.name} (${selected.mcVersion})` : "Versions"}</h3>
          {versions && versions.length === 0 && <Alert tone="warning">No version of this project supports this profile's Minecraft version and loader.</Alert>}
          {versions && versions.length > 0 && (
            <table className="table">
              <thead><tr><th>Version</th><th>Type</th><th>Minecraft</th><th>Loaders</th><th>Published</th><th>Dependencies</th><th /></tr></thead>
              <tbody>
                {versions.slice(0, 12).map((v) => (
                  <tr key={v.id}>
                    <td>{v.version_number}</td>
                    <td><Badge tone={v.version_type === "release" ? "accent" : "warning"}>{v.version_type}</Badge></td>
                    <td className="small">{v.game_versions.slice(-3).join(", ")}</td>
                    <td className="small">{v.loaders.join(", ")}</td>
                    <td className="small">{relativeTime(v.date_published)}</td>
                    <td className="small">{v.dependencies.filter((d) => d.dependency_type === "required").length} required</td>
                    <td><Button size="sm" onClick={() => onInstall(v.id)}>Install</Button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
      {cf && (
        <div className="row" style={{ alignItems: "flex-start" }}>
          <Img src={cf.logo?.thumbnailUrl} size={72} />
          <div className="grow">
            <p>{cf.summary}</p>
            <div className="kv mt">
              <span>Authors</span><span>{cf.authors.map((a) => a.name).join(", ")}</span>
              <span>Downloads</span><span>{count(cf.downloadCount)}</span>
              <span>Updated</span><span>{relativeTime(cf.dateModified)}</span>
              <span>Third-party downloads</span><span>{cf.allowModDistribution === false ? "Not allowed by the author (manual download)" : "Allowed"}</span>
              <span>Recent files</span><span>{cf.latestFilesIndexes.slice(0, 6).map((f) => f.gameVersion).filter((v, i, a) => a.indexOf(v) === i).join(", ")}</span>
              <span>Source</span><span>CurseForge</span>
            </div>
          </div>
        </div>
      )}
    </Modal>
  );
}
