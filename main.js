// Fills in download sizes, version and checksums from downloads/files.json (written by
// `npm run website`), and opens screenshots in a lightbox. Download links point at
// files.json's "base" (the latest GitHub release) or, without one, at downloads/ next to the page.

(async () => {
  try {
    const res = await fetch("downloads/files.json", { cache: "no-cache" });
    if (!res.ok) return;
    const info = await res.json();
    const mb = (n) => `${(n / 1048576).toFixed(1)} MB`;
    for (const el of document.querySelectorAll("[data-version]")) el.textContent = info.version;
    for (const [key, file] of Object.entries(info.files)) {
      for (const a of document.querySelectorAll(`[data-download="${key}"]`)) a.href = `${info.base ?? "downloads/"}${file.name}`;
      for (const el of document.querySelectorAll(`[data-size="${key}"]`)) el.textContent = el.closest(".meta") ? `Installer ${mb(file.size)}` : mb(file.size);
    }
    const list = document.querySelector("[data-hashes]");
    if (list) {
      list.replaceChildren(...Object.values(info.files).map((f) => {
        const li = document.createElement("li");
        const code = document.createElement("code");
        code.textContent = f.sha256;
        li.append(`${f.name}: `, code);
        return li;
      }));
    }
  } catch {
    // opened straight from disk or files not published yet: the static text stays
  }
})();

const box = document.querySelector(".lightbox");
const boxImg = box.querySelector("img");
for (const shot of document.querySelectorAll(".shot")) {
  shot.addEventListener("click", () => {
    boxImg.src = shot.dataset.full;
    boxImg.alt = shot.querySelector("span")?.textContent ?? "";
    box.hidden = false;
  });
}
box.addEventListener("click", (e) => {
  if (e.target !== boxImg) box.hidden = true;
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") box.hidden = true;
});
