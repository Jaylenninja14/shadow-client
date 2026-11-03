//! Download manager.
//!
//! * A *job* groups many files ("Install Minecraft 1.21.11"). Jobs run with bounded concurrency.
//! * Each file is written to `<dest>.part`, resumed with HTTP Range when possible, verified
//!   against SHA-1/SHA-512/size, and only then renamed into place.
//! * Files that already verify are skipped (no redundant downloads).
//! * Progress is pushed as `download:progress` events at most ~4 times per second per job.
//! * Jobs can be cancelled (partial files are kept for resume) and retried.
//! * Nothing downloaded is ever executed by this module.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::header::{RANGE, USER_AGENT};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio_util::sync::CancellationToken;

use crate::events::{emit, Sink};
use crate::http::Http;
use crate::{util, Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DownloadItem {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub size: Option<u64>,
}

impl DownloadItem {
    pub fn new(url: impl Into<String>, dest: impl Into<PathBuf>) -> Self {
        Self { url: url.into(), dest: dest.into(), sha1: None, sha512: None, size: None }
    }
    pub fn sha1(mut self, h: Option<String>) -> Self {
        self.sha1 = h;
        self
    }
    pub fn sha512(mut self, h: Option<String>) -> Self {
        self.sha512 = h;
        self
    }
    pub fn size(mut self, s: Option<u64>) -> Self {
        self.size = s;
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInfo {
    pub id: String,
    pub title: String,
    pub status: JobStatus,
    pub total_files: usize,
    pub done_files: usize,
    pub skipped_files: usize,
    /// Sum of known sizes; `None` if any size is unknown.
    pub total_bytes: Option<u64>,
    pub downloaded_bytes: u64,
    pub bytes_per_sec: u64,
    pub error: Option<String>,
    pub failed: Vec<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub retryable: bool,
}

struct JobRuntime {
    info: Mutex<JobInfo>,
    items: Vec<DownloadItem>,
    cancel: CancellationToken,
    downloaded: AtomicU64,
    done: AtomicUsize,
    skipped: AtomicUsize,
}

pub struct DownloadManager {
    http: Arc<Http>,
    sink: Sink,
    jobs: Mutex<HashMap<String, Arc<JobRuntime>>>,
    order: Mutex<Vec<String>>,
    concurrency: AtomicUsize,
    retries: AtomicU32,
    history_file: Option<PathBuf>,
}

impl DownloadManager {
    pub fn new(http: Arc<Http>, sink: Sink, history_file: Option<PathBuf>) -> Self {
        let mgr = Self {
            http,
            sink,
            jobs: Mutex::new(HashMap::new()),
            order: Mutex::new(Vec::new()),
            concurrency: AtomicUsize::new(6),
            retries: AtomicU32::new(3),
            history_file,
        };
        mgr.load_history();
        mgr
    }

    pub fn set_limits(&self, concurrency: u32, retries: u32) {
        self.concurrency.store(concurrency.clamp(1, 16) as usize, Ordering::Relaxed);
        self.retries.store(retries.min(10), Ordering::Relaxed);
    }

    fn load_history(&self) {
        let Some(path) = &self.history_file else { return };
        let Ok(Some(list)) = util::read_json_opt::<Vec<JobInfo>>(path) else { return };
        let mut jobs = self.jobs.lock().unwrap();
        let mut order = self.order.lock().unwrap();
        for mut info in list {
            // A job that was running when the app closed is reported as cancelled.
            if matches!(info.status, JobStatus::Running | JobStatus::Queued) {
                info.status = JobStatus::Cancelled;
                info.error = Some("Interrupted when the launcher closed".into());
            }
            info.retryable = false; // items are not persisted; history is informational
            order.push(info.id.clone());
            jobs.insert(
                info.id.clone(),
                Arc::new(JobRuntime {
                    info: Mutex::new(info),
                    items: vec![],
                    cancel: CancellationToken::new(),
                    downloaded: AtomicU64::new(0),
                    done: AtomicUsize::new(0),
                    skipped: AtomicUsize::new(0),
                }),
            );
        }
    }

    fn save_history(&self) {
        let Some(path) = &self.history_file else { return };
        let list: Vec<JobInfo> = self.list().into_iter().filter(|j| !matches!(j.status, JobStatus::Running | JobStatus::Queued)).take(100).collect();
        let _ = util::write_json(path, &list);
    }

    pub fn list(&self) -> Vec<JobInfo> {
        let jobs = self.jobs.lock().unwrap();
        let order = self.order.lock().unwrap();
        order.iter().rev().filter_map(|id| jobs.get(id)).map(|j| j.info.lock().unwrap().clone()).collect()
    }

    pub fn cancel(&self, id: &str) -> bool {
        if let Some(j) = self.jobs.lock().unwrap().get(id) {
            j.cancel.cancel();
            return true;
        }
        false
    }

    pub fn clear_finished(&self) {
        let mut jobs = self.jobs.lock().unwrap();
        let mut order = self.order.lock().unwrap();
        order.retain(|id| jobs.get(id).is_some_and(|j| matches!(j.info.lock().unwrap().status, JobStatus::Running | JobStatus::Queued)));
        jobs.retain(|id, _| order.contains(id));
        drop(jobs);
        drop(order);
        self.save_history();
    }

    /// Re-run a failed or cancelled job. Verified files are skipped, partial files resumed.
    pub async fn retry(&self, id: &str) -> Result<JobInfo> {
        let (title, items) = {
            let jobs = self.jobs.lock().unwrap();
            let j = jobs.get(id).ok_or_else(|| Error::NotFound(format!("download job {id}")))?;
            let info = j.info.lock().unwrap();
            if !info.retryable || j.items.is_empty() {
                return Err(Error::Invalid("This download can no longer be retried from here; start the action again".into()));
            }
            if matches!(info.status, JobStatus::Running | JobStatus::Queued) {
                return Err(Error::Invalid("This download is still running".into()));
            }
            (info.title.clone(), j.items.clone())
        };
        self.run(&title, items).await
    }

    /// Run a job to completion. Returns the final job info; `Err` if any file failed.
    pub async fn run(&self, title: &str, items: Vec<DownloadItem>) -> Result<JobInfo> {
        self.run_with_token(title, items, CancellationToken::new()).await
    }

    pub async fn run_with_token(&self, title: &str, mut items: Vec<DownloadItem>, cancel: CancellationToken) -> Result<JobInfo> {
        // Deduplicate destinations (the same library can be listed twice).
        let mut seen = std::collections::HashSet::new();
        items.retain(|i| seen.insert(i.dest.clone()));
        for it in &items {
            Http::ensure_url_allowed(&it.url)?;
        }

        let total_bytes = items.iter().map(|i| i.size).sum::<Option<u64>>();
        // Disk-space check (with 5 % / 50 MB headroom) on the volume of the first destination.
        if let (Some(total), Some(first)) = (total_bytes, items.first()) {
            if let Some(free) = util::free_space(&first.dest) {
                let needed = total + (total / 20).max(50 * 1024 * 1024);
                if free < needed {
                    return Err(Error::msg(format!(
                        "Not enough disk space: {} MB needed, {} MB free",
                        needed / 1_048_576,
                        free / 1_048_576
                    )));
                }
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        let job = Arc::new(JobRuntime {
            info: Mutex::new(JobInfo {
                id: id.clone(),
                title: title.to_string(),
                status: JobStatus::Running,
                total_files: items.len(),
                done_files: 0,
                skipped_files: 0,
                total_bytes,
                downloaded_bytes: 0,
                bytes_per_sec: 0,
                error: None,
                failed: vec![],
                started_at: util::now_iso(),
                finished_at: None,
                retryable: true,
            }),
            items: items.clone(),
            cancel,
            downloaded: AtomicU64::new(0),
            done: AtomicUsize::new(0),
            skipped: AtomicUsize::new(0),
        });
        self.jobs.lock().unwrap().insert(id.clone(), job.clone());
        self.order.lock().unwrap().push(id.clone());

        // Throttled progress reporter.
        let reporter = {
            let job = job.clone();
            let sink = self.sink.clone();
            tokio::spawn(async move {
                let mut last_bytes = 0u64;
                let mut last_t = Instant::now();
                let mut speed = 0f64;
                loop {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    let bytes = job.downloaded.load(Ordering::Relaxed);
                    let dt = last_t.elapsed().as_secs_f64().max(0.001);
                    let inst = (bytes.saturating_sub(last_bytes)) as f64 / dt;
                    speed = if speed == 0.0 { inst } else { speed * 0.7 + inst * 0.3 };
                    last_bytes = bytes;
                    last_t = Instant::now();
                    let snapshot = {
                        let mut info = job.info.lock().unwrap();
                        info.downloaded_bytes = bytes;
                        info.done_files = job.done.load(Ordering::Relaxed);
                        info.skipped_files = job.skipped.load(Ordering::Relaxed);
                        info.bytes_per_sec = speed as u64;
                        info.clone()
                    };
                    if snapshot.status != JobStatus::Running {
                        break;
                    }
                    emit(&sink, "download:progress", &snapshot);
                }
            })
        };

        let concurrency = self.concurrency.load(Ordering::Relaxed).max(1);
        let retries = self.retries.load(Ordering::Relaxed);
        let results: Vec<(DownloadItem, Result<bool>)> = futures_util::stream::iter(items.into_iter())
            .map(|item| {
                let job = job.clone();
                let http = self.http.clone();
                async move {
                    let r = download_one(&http, &item, &job, retries).await;
                    match &r {
                        Ok(skipped) => {
                            job.done.fetch_add(1, Ordering::Relaxed);
                            if *skipped {
                                job.skipped.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Err(e) => tracing::warn!("download failed {}: {e}", item.url),
                    }
                    (item, r)
                }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        let cancelled = job.cancel.is_cancelled();
        let failed: Vec<(DownloadItem, Error)> = results.into_iter().filter_map(|(i, r)| r.err().map(|e| (i, e))).collect();
        let final_info = {
            let mut info = job.info.lock().unwrap();
            info.downloaded_bytes = job.downloaded.load(Ordering::Relaxed);
            info.done_files = job.done.load(Ordering::Relaxed);
            info.skipped_files = job.skipped.load(Ordering::Relaxed);
            info.finished_at = Some(util::now_iso());
            info.bytes_per_sec = 0;
            if cancelled {
                info.status = JobStatus::Cancelled;
                info.error = Some("Cancelled by user".into());
            } else if !failed.is_empty() {
                info.status = JobStatus::Failed;
                info.failed = failed.iter().map(|(i, e)| format!("{}: {e}", file_label(&i.dest))).take(50).collect();
                info.error = Some(format!("{} of {} files failed. First error: {}", failed.len(), info.total_files, failed[0].1));
            } else {
                info.status = JobStatus::Completed;
            }
            info.clone()
        };
        let _ = reporter.await;
        emit(&self.sink, "download:progress", &final_info);
        self.save_history();

        match final_info.status {
            JobStatus::Completed => Ok(final_info),
            JobStatus::Cancelled => Err(Error::Cancelled),
            _ => Err(Error::msg(final_info.error.clone().unwrap_or_else(|| "Download failed".into()))),
        }
    }
}

fn file_label(p: &Path) -> String {
    p.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default()
}

fn part_path(dest: &Path) -> PathBuf {
    let mut s = dest.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

/// Download one item. Returns Ok(true) if skipped because it already verified.
async fn download_one(http: &Http, item: &DownloadItem, job: &JobRuntime, retries: u32) -> Result<bool> {
    if job.cancel.is_cancelled() {
        return Err(Error::Cancelled);
    }
    let dest = item.dest.clone();
    let (s1, s512, size) = (item.sha1.clone(), item.sha512.clone(), item.size);
    let verified = {
        let dest = dest.clone();
        tokio::task::spawn_blocking(move || util::file_matches(&dest, s1.as_deref(), s512.as_deref(), size)).await.unwrap_or(false)
    };
    if verified {
        if let Some(sz) = item.size {
            job.downloaded.fetch_add(sz, Ordering::Relaxed);
        }
        return Ok(true);
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut attempt = 0;
    loop {
        attempt += 1;
        match try_download(http, item, job).await {
            Ok(()) => return Ok(false),
            Err(Error::Cancelled) => return Err(Error::Cancelled),
            Err(e) => {
                let permanent = matches!(e, Error::NotFound(_) | Error::Auth(_) | Error::UnsafePath(_));
                if permanent || attempt > retries {
                    return Err(e);
                }
                let backoff = Duration::from_millis(500 * 2u64.pow(attempt.min(5) - 1));
                tokio::select! {
                    _ = tokio::time::sleep(backoff) => {}
                    _ = job.cancel.cancelled() => return Err(Error::Cancelled),
                }
            }
        }
    }
}

async fn try_download(http: &Http, item: &DownloadItem, job: &JobRuntime) -> Result<()> {
    let part = part_path(&item.dest);
    let mut existing = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
    if let Some(sz) = item.size {
        if existing > sz {
            let _ = tokio::fs::remove_file(&part).await;
            existing = 0;
        }
    }
    let mut req = http.client.get(&item.url).header(USER_AGENT, http.user_agent());
    if existing > 0 {
        req = req.header(RANGE, format!("bytes={existing}-"));
    }
    let resp = tokio::select! {
        r = req.send() => r?,
        _ = job.cancel.cancelled() => return Err(Error::Cancelled),
    };
    let status = resp.status();
    if status.as_u16() == 404 || status.as_u16() == 410 {
        return Err(Error::NotFound(item.url.clone()));
    }
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(Error::Auth(format!("Access denied downloading {}", file_label(&item.dest))));
    }
    if !status.is_success() {
        return Err(Error::Http(format!("HTTP {status} for {}", file_label(&item.dest))));
    }
    let resumed = existing > 0 && status.as_u16() == 206;
    let mut file = if resumed {
        job.downloaded.fetch_add(existing, Ordering::Relaxed);
        tokio::fs::OpenOptions::new().append(true).open(&part).await?
    } else {
        tokio::fs::File::create(&part).await?
    };
    let mut written_now = 0u64;
    let mut stream = resp.bytes_stream();
    loop {
        let next = tokio::select! {
            n = stream.next() => n,
            _ = job.cancel.cancelled() => {
                file.flush().await.ok();
                // keep the .part file so the job can resume
                job.downloaded.fetch_sub(written_now + if resumed { existing } else { 0 }, Ordering::Relaxed);
                return Err(Error::Cancelled);
            }
        };
        let Some(chunk) = next else { break };
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                file.flush().await.ok();
                job.downloaded.fetch_sub(written_now + if resumed { existing } else { 0 }, Ordering::Relaxed);
                return Err(e.into());
            }
        };
        file.write_all(&chunk).await?;
        written_now += chunk.len() as u64;
        job.downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
    }
    file.flush().await?;
    file.sync_all().await.ok();
    drop(file);

    let (part2, s1, s512, size) = (part.clone(), item.sha1.clone(), item.sha512.clone(), item.size);
    let verify = tokio::task::spawn_blocking(move || util::verify_file(&part2, s1.as_deref(), s512.as_deref(), size))
        .await
        .map_err(|e| Error::msg(e.to_string()))?;
    if let Err(e) = verify {
        let _ = tokio::fs::remove_file(&part).await;
        job.downloaded.fetch_sub(written_now + if resumed { existing } else { 0 }, Ordering::Relaxed);
        return Err(e);
    }
    if tokio::fs::metadata(&item.dest).await.is_ok() {
        let _ = tokio::fs::remove_file(&item.dest).await;
    }
    tokio::fs::rename(&part, &item.dest).await?;
    Ok(())
}

/// Convenience for single small files outside the job system (e.g. version JSONs).
pub async fn fetch_to_file(http: &Http, item: &DownloadItem) -> Result<()> {
    if util::file_matches(&item.dest, item.sha1.as_deref(), item.sha512.as_deref(), item.size) {
        return Ok(());
    }
    Http::ensure_url_allowed(&item.url)?;
    let job = JobRuntime {
        info: Mutex::new(JobInfo {
            id: String::new(),
            title: String::new(),
            status: JobStatus::Running,
            total_files: 1,
            done_files: 0,
            skipped_files: 0,
            total_bytes: None,
            downloaded_bytes: 0,
            bytes_per_sec: 0,
            error: None,
            failed: vec![],
            started_at: String::new(),
            finished_at: None,
            retryable: false,
        }),
        items: vec![],
        cancel: CancellationToken::new(),
        downloaded: AtomicU64::new(0),
        done: AtomicUsize::new(0),
        skipped: AtomicUsize::new(0),
    };
    if let Some(parent) = item.dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut attempt = 0;
    loop {
        attempt += 1;
        match try_download(http, item, &job).await {
            Ok(()) => return Ok(()),
            Err(e) if attempt >= 3 || matches!(e, Error::NotFound(_) | Error::HashMismatch { .. }) => return Err(e),
            Err(_) => tokio::time::sleep(Duration::from_millis(500 * attempt)).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Minimal HTTP/1.1 server: serves `body`, honours `Range: bytes=N-`. The first request is
    /// cut off after `cut_after` bytes (simulated network drop). `delay_ms` slows every chunk.
    async fn serve(body: Vec<u8>, cut_after: Option<usize>, delay_ms: u64) -> (String, Arc<AtomicUsize>, Arc<Mutex<Vec<String>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let hits = Arc::new(AtomicUsize::new(0));
        let ranges = Arc::new(Mutex::new(vec![]));
        let (h2, r2) = (hits.clone(), ranges.clone());
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else { break };
                let body = body.clone();
                let n = h2.fetch_add(1, Ordering::SeqCst);
                let r2 = r2.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let len = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..len]).to_string();
                    let start = req.lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("range: bytes=").map(|v| v.trim_end_matches('-').parse::<usize>().unwrap_or(0)));
                    r2.lock().unwrap().push(start.map(|s| s.to_string()).unwrap_or_default());
                    let (status, slice) = match start {
                        Some(s) => (format!("HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {s}-{}/{}\r\n", body.len() - 1, body.len()), &body[s..]),
                        None => ("HTTP/1.1 200 OK\r\nAccept-Ranges: bytes\r\n".to_string(), &body[..]),
                    };
                    let head = format!("{status}Content-Length: {}\r\nConnection: close\r\n\r\n", slice.len());
                    let _ = sock.write_all(head.as_bytes()).await;
                    let limit = if n == 0 { cut_after.unwrap_or(usize::MAX) } else { usize::MAX };
                    for chunk in slice[..slice.len().min(limit)].chunks(16 * 1024) {
                        if sock.write_all(chunk).await.is_err() {
                            return;
                        }
                        if delay_ms > 0 {
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        }
                    }
                    let _ = sock.shutdown().await;
                });
            }
        });
        (format!("http://{addr}/file.bin"), hits, ranges)
    }

    fn body() -> Vec<u8> {
        (0..400_000u32).map(|i| (i * 7 % 251) as u8).collect()
    }

    fn manager(dir: &Path) -> DownloadManager {
        let http = Arc::new(Http::new(dir.join("cache"), "test".into()).unwrap());
        let m = DownloadManager::new(http, crate::events::null_sink(), Some(dir.join("history.json")));
        m.set_limits(4, 3);
        m
    }

    #[tokio::test]
    async fn resumes_after_interruption_and_verifies() {
        let b = body();
        let (url, hits, ranges) = serve(b.clone(), Some(150_000), 0).await;
        let d = tempfile::tempdir().unwrap();
        let m = manager(d.path());
        let dest = d.path().join("out").join("file.bin");
        let item = DownloadItem::new(url, dest.clone()).sha1(Some(util::sha1_hex(&b))).size(Some(b.len() as u64));
        let info = m.run("resume test", vec![item.clone()]).await.unwrap();
        assert_eq!(info.status, JobStatus::Completed);
        assert_eq!(std::fs::read(&dest).unwrap(), b);
        assert!(hits.load(Ordering::SeqCst) >= 2, "a second request was needed");
        let r = ranges.lock().unwrap().clone();
        assert!(r.iter().skip(1).any(|x| x.parse::<usize>().unwrap_or(0) > 0), "second request resumed with a Range header: {r:?}");
        assert!(!part_path(&dest).exists());

        // Running again downloads nothing: the file already verifies.
        let before = hits.load(Ordering::SeqCst);
        let info = m.run("again", vec![item]).await.unwrap();
        assert_eq!(info.skipped_files, 1);
        assert_eq!(hits.load(Ordering::SeqCst), before);
        // History persisted.
        assert!(d.path().join("history.json").exists());
    }

    #[tokio::test]
    async fn hash_mismatch_fails_and_leaves_no_file() {
        let b = body();
        let (url, _, _) = serve(b.clone(), None, 0).await;
        let d = tempfile::tempdir().unwrap();
        let m = manager(d.path());
        m.set_limits(1, 1);
        let dest = d.path().join("bad.bin");
        let err = m.run("bad", vec![DownloadItem::new(url, dest.clone()).sha1(Some("0".repeat(40)))]).await.unwrap_err();
        assert!(err.to_string().contains("failed"), "{err}");
        assert!(!dest.exists() && !part_path(&dest).exists());
        let jobs = m.list();
        assert_eq!(jobs[0].status, JobStatus::Failed);
        assert!(jobs[0].retryable);
    }

    #[tokio::test]
    async fn cancel_keeps_partial_then_retry_completes() {
        let b = body();
        let (url, _, _) = serve(b.clone(), None, 40).await;
        let d = tempfile::tempdir().unwrap();
        let m = Arc::new(manager(d.path()));
        let dest = d.path().join("slow.bin");
        let item = DownloadItem::new(url, dest.clone()).sha1(Some(util::sha1_hex(&b))).size(Some(b.len() as u64));
        let m2 = m.clone();
        let task = tokio::spawn(async move { m2.run("slow", vec![item]).await });
        tokio::time::sleep(Duration::from_millis(250)).await;
        let id = m.list()[0].id.clone();
        assert!(m.cancel(&id));
        assert!(matches!(task.await.unwrap(), Err(Error::Cancelled)));
        assert_eq!(m.list()[0].status, JobStatus::Cancelled);
        assert!(part_path(&dest).exists(), "partial data kept for resume");
        let info = m.retry(&id).await.unwrap();
        assert_eq!(info.status, JobStatus::Completed);
        assert_eq!(std::fs::read(&dest).unwrap(), b);
    }

    #[tokio::test]
    async fn refuses_insecure_urls() {
        let d = tempfile::tempdir().unwrap();
        let m = manager(d.path());
        assert!(m.run("x", vec![DownloadItem::new("http://example.com/a", d.path().join("a"))]).await.is_err());
    }
}
