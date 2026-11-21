// Fills in download sizes, version and checksums from downloads/files.json (written by
// `npm run website`), and opens screenshots in a lightbox. Download links point at
// files.json's "base" (the latest GitHub release) or, without one, at downloads/ next to the page.

// ============================================================ Download info
(async () => {
  try {
    const res = await fetch("downloads/files.json", { cache: "no-cache" });
    if (!res.ok) return;
    const info = await res.json();
    const mb = (n) => `${(n / 1048576).toFixed(1)} MB`;

    // Update version
    for (const el of document.querySelectorAll("[data-version]")) {
      el.textContent = info.version;
    }

    // Update download links and sizes
    for (const [key, file] of Object.entries(info.files)) {
      for (const a of document.querySelectorAll(`[data-download="${key}"]`)) {
        a.href = `${info.base ?? "downloads/"}${file.name}`;
      }
      for (const el of document.querySelectorAll(`[data-size="${key}"]`)) {
        el.textContent = el.closest(".meta") ? `Installer ${mb(file.size)}` : mb(file.size);
      }
    }

    // Update checksums
    const list = document.querySelector("[data-hashes]");
    if (list) {
      list.replaceChildren(...Object.values(info.files).map((f) => {
        const li = document.createElement("li");
        const code = document.createElement("code");
        code.textContent = f.sha256;
        li.append(`${f.name}: `, code);
        return li;
      }));

      // Show copy button
      const copyBtn = document.querySelector(".copy-hashes");
      if (copyBtn) {
        copyBtn.style.display = "inline-flex";
      }
    }
  } catch {
    // opened straight from disk or files not published yet: the static text stays
  }
})();

// ============================================================ Archive
// downloads/releases.json (written by `npm run release`): every published version with its files.
(async () => {
  const box = document.querySelector("[data-archive]");
  if (!box) return;
  try {
    const res = await fetch("downloads/releases.json", { cache: "no-cache" });
    if (!res.ok) return;
    const releases = await res.json();
    if (!Array.isArray(releases) || releases.length === 0) return;
    // releases before the rename to Ghost Client used Shadow-Client-* names
    const kinds = [["-Setup.exe", "Installer"], [".msi", "MSI"], ["-Portable.zip", "Portable"]];
    const mb = (n) => `${(n / 1048576).toFixed(1)} MB`;
    const link = (href, text, title) => {
      const a = document.createElement("a");
      a.href = href;
      a.textContent = text;
      if (title) a.title = title;
      return a;
    };
    const rows = releases.map((r, i) => {
      const row = document.createElement("div");
      row.className = "archive-row";
      const name = document.createElement("strong");
      name.textContent = `Ghost Client ${r.version.replace(/^v/, "")}`;
      if (i === 0) {
        const tag = document.createElement("span");
        tag.className = "archive-latest";
        tag.textContent = "Latest";
        name.append(" ", tag);
      }
      const date = document.createElement("span");
      date.className = "archive-date";
      date.textContent = new Date(r.date).toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" });
      const files = document.createElement("span");
      files.className = "archive-files";
      for (const [file, label] of kinds) {
        const asset = ["Ghost-Client", "Shadow-Client"].map((p) => (r.assets || []).find((a) => a.name === p + file)).find(Boolean);
        if (asset) files.append(link(asset.url, label, mb(asset.size)));
      }
      files.append(link(r.url, "Notes"));
      row.append(name, date, files);
      return row;
    });
    box.replaceChildren(...rows);
  } catch {
    // opened straight from disk or not published yet: the link to GitHub Releases stays
  }
})();

// ============================================================ Scroll progress indicator
const scrollProgress = document.querySelector(".scroll-progress");
window.addEventListener("scroll", () => {
  const windowHeight = document.documentElement.scrollHeight - document.documentElement.clientHeight;
  const scrolled = (window.scrollY / windowHeight) * 100;
  scrollProgress.style.width = `${Math.min(scrolled, 100)}%`;
  scrollProgress.setAttribute("aria-valuenow", Math.round(scrolled));
});

// ============================================================ Mobile menu toggle
const mobileMenuToggle = document.querySelector(".mobile-menu-toggle");
const mobileNav = document.querySelector(".mobile-nav");

if (mobileMenuToggle && mobileNav) {
  mobileMenuToggle.addEventListener("click", () => {
    const isExpanded = mobileMenuToggle.getAttribute("aria-expanded") === "true";
    mobileMenuToggle.setAttribute("aria-expanded", !isExpanded);
    mobileNav.hidden = isExpanded;
  });

  // Close mobile menu when clicking a link
  mobileNav.querySelectorAll("a").forEach(link => {
    link.addEventListener("click", () => {
      mobileMenuToggle.setAttribute("aria-expanded", "false");
      mobileNav.hidden = true;
    });
  });
}

// ============================================================ Back to top button
const backToTop = document.querySelector(".back-to-top");
if (backToTop) {
  window.addEventListener("scroll", () => {
    if (window.scrollY > 500) {
      backToTop.hidden = false;
    } else {
      backToTop.hidden = true;
    }
  });

  backToTop.addEventListener("click", () => {
    window.scrollTo({ top: 0, behavior: "smooth" });
  });
}

// ============================================================ Toast notifications
function showToast(message, type = "success") {
  const container = document.querySelector(".toast-container");
  const toast = document.createElement("div");
  toast.className = `toast ${type}`;
  toast.textContent = message;
  container.appendChild(toast);

  setTimeout(() => {
    toast.classList.add("toast-out");
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

// ============================================================ Copy checksums button
const copyHashesBtn = document.querySelector(".copy-hashes");
if (copyHashesBtn) {
  copyHashesBtn.addEventListener("click", async () => {
    const list = document.querySelector("[data-hashes]");
    const items = Array.from(list.querySelectorAll("li"));
    const text = items.map(li => li.textContent).join("\n");

    try {
      await navigator.clipboard.writeText(text);
      showToast("Checksums copied to clipboard!");
    } catch (err) {
      showToast("Failed to copy checksums", "error");
    }
  });
}

// ============================================================ Screenshot lightbox
const box = document.querySelector(".lightbox");
const boxImg = box.querySelector("img");
const shots = Array.from(document.querySelectorAll(".shot"));
let currentIndex = 0;

function openLightbox(index) {
  currentIndex = index;
  const shot = shots[index];
  boxImg.src = shot.dataset.full;
  boxImg.alt = shot.querySelector("span")?.textContent ?? "";
  box.hidden = false;
  document.body.style.overflow = "hidden";
}

function closeLightbox() {
  box.hidden = true;
  document.body.style.overflow = "";
}

function showNextImage() {
  currentIndex = (currentIndex + 1) % shots.length;
  openLightbox(currentIndex);
}

function showPrevImage() {
  currentIndex = (currentIndex - 1 + shots.length) % shots.length;
  openLightbox(currentIndex);
}

// Click on screenshot to open
shots.forEach((shot, index) => {
  shot.addEventListener("click", () => openLightbox(index));
});

// Close button
box.querySelector(".lightbox-close").addEventListener("click", closeLightbox);

// Navigation buttons
box.querySelector(".lightbox-next").addEventListener("click", showNextImage);
box.querySelector(".lightbox-prev").addEventListener("click", showPrevImage);

// Click outside image to close
box.addEventListener("click", (e) => {
  if (e.target !== boxImg && !e.target.closest(".lightbox-nav") && !e.target.closest(".lightbox-close")) {
    closeLightbox();
  }
});

// Keyboard navigation
document.addEventListener("keydown", (e) => {
  if (box.hidden) return;

  switch (e.key) {
    case "Escape":
      closeLightbox();
      break;
    case "ArrowRight":
      showNextImage();
      break;
    case "ArrowLeft":
      showPrevImage();
      break;
  }
});

// ============================================================ Smooth scroll for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
  anchor.addEventListener("click", function(e) {
    const href = this.getAttribute("href");
    if (href === "#" || href === "#top") return;

    const target = document.querySelector(href);
    if (target) {
      e.preventDefault();
      target.scrollIntoView({ behavior: "smooth", block: "start" });

      // Update URL without jumping
      history.pushState(null, null, href);
    }
  });
});

// ============================================================ Loading state for dynamic content
window.addEventListener("load", () => {
  document.body.classList.add("loaded");
});

// ============================================================ Enhanced accessibility
// Add role="region" and aria-label to major sections for screen readers
document.querySelectorAll(".section").forEach((section, index) => {
  if (section.id) {
    section.setAttribute("role", "region");
    const heading = section.querySelector("h2");
    if (heading) {
      section.setAttribute("aria-labelledby", section.id + "-heading");
      heading.id = section.id + "-heading";
    }
  }
});

// ============================================================ Performance: Lazy load images below fold
if ("IntersectionObserver" in window) {
  const imageObserver = new IntersectionObserver((entries, observer) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const img = entry.target;
        if (img.dataset.src) {
          img.src = img.dataset.src;
          img.removeAttribute("data-src");
        }
        observer.unobserve(img);
      }
    });
  });

  document.querySelectorAll("img[loading='lazy']").forEach(img => {
    imageObserver.observe(img);
  });
}

// ============================================================ Light and dark
document.body.classList.remove("no-js");
const themeToggle = document.querySelector(".theme-toggle");
if (themeToggle) {
  themeToggle.addEventListener("click", () => {
    const next = document.documentElement.dataset.theme === "light" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    try { localStorage.setItem("ghost-theme", next); } catch { /* private window: just this visit */ }
  });
}

// ============================================================ Download menu (Installer / MSI / Portable)
for (const menu of document.querySelectorAll(".dl-menu")) {
  const more = menu.querySelector(".dl-more");
  const set = (open) => { menu.classList.toggle("open", open); more.setAttribute("aria-expanded", String(open)); };
  more.addEventListener("click", (e) => { e.stopPropagation(); set(!menu.classList.contains("open")); });
  document.addEventListener("click", (e) => { if (!menu.contains(e.target)) set(false); });
  document.addEventListener("keydown", (e) => { if (e.key === "Escape") set(false); });
}

// ============================================================ Showcases: tabs, arrows, auto-advance
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
for (const show of document.querySelectorAll("[data-showcase]")) {
  const tabs = Array.from(show.querySelectorAll(".tab"));
  const slides = Array.from(show.querySelectorAll(".slide"));
  const copy = show.querySelector(".stage-copy");
  const bar = show.querySelector(".stage-progress");
  let data = [];
  try { data = JSON.parse(show.querySelector(".stage-data").textContent); } catch { /* copy stays on the first slide */ }
  const SPAN = 7000;
  let current = 0, started = performance.now(), paused = false, visible = false;
  const go = (i) => {
    current = (i + slides.length) % slides.length;
    slides.forEach((s, k) => s.classList.toggle("on", k === current));
    tabs.forEach((t, k) => { t.classList.toggle("on", k === current); t.setAttribute("aria-selected", String(k === current)); });
    const d = data[current];
    if (d && copy) {
      copy.classList.add("swap");
      setTimeout(() => {
        copy.querySelector("h3").innerHTML = `${d.a} <b>${d.b}</b>`;
        copy.querySelector("p").innerHTML = d.p;
        copy.classList.remove("swap");
      }, 220);
    }
    started = performance.now();
  };
  tabs.forEach((t, i) => t.addEventListener("click", () => go(i)));
  show.querySelectorAll(".stage-arrow").forEach((b) => b.addEventListener("click", () => go(current + Number(b.dataset.dir))));
  const frame = show.querySelector(".stage-frame");
  frame.addEventListener("mouseenter", () => { paused = true; });
  frame.addEventListener("mouseleave", () => { paused = false; started = performance.now(); });
  if ("IntersectionObserver" in window) {
    new IntersectionObserver(([e]) => { visible = e.isIntersecting; if (visible) started = performance.now(); }, { threshold: 0.35 }).observe(frame);
  }
  if (!reduceMotion) {
    const tick = (now) => {
      if (visible && !paused) {
        const f = (now - started) / SPAN;
        if (f >= 1) go(current + 1);
        else if (bar) bar.style.width = `${f * 100}%`;
      }
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }
}

// ============================================================ The hero window straightens as you scroll
const heroWindow = document.querySelector(".hero-window");
if (heroWindow && !reduceMotion) {
  const tilt = () => {
    const r = heroWindow.getBoundingClientRect();
    const f = Math.min(Math.max((window.innerHeight - r.top) / (window.innerHeight * 0.9), 0), 1);
    heroWindow.style.setProperty("--tilt", `${(18 * (1 - f)).toFixed(2)}deg`);
  };
  tilt();
  window.addEventListener("scroll", tilt, { passive: true });
  window.addEventListener("resize", tilt);
}

// ============================================================ Things fade in as they scroll into view
const reveals = document.querySelectorAll(".reveal, .perf");
if ("IntersectionObserver" in window && !reduceMotion) {
  const io = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      // cards in one row come in one after another
      const siblings = Array.from(e.target.parentElement.children);
      e.target.style.transitionDelay = `${Math.min(siblings.indexOf(e.target) % 6, 5) * 70}ms`;
      e.target.classList.add("in");
      io.unobserve(e.target);
    }
  }, { threshold: 0.15, rootMargin: "0px 0px -40px 0px" });
  reveals.forEach((el) => io.observe(el));
  // a fallback in case the observer is held back (a background tab, some browsers): anything scrolled past shows
  const late = () => {
    let left = 0;
    for (const el of reveals) {
      if (el.classList.contains("in")) continue;
      if (el.getBoundingClientRect().top < window.innerHeight) { el.classList.add("in"); io.unobserve(el); } else left++;
    }
    if (!left) window.removeEventListener("scroll", onScroll);
  };
  let lateTimer = 0;
  const onScroll = () => { clearTimeout(lateTimer); lateTimer = setTimeout(late, 400); };
  window.addEventListener("scroll", onScroll, { passive: true });
} else {
  reveals.forEach((el) => el.classList.add("in"));
}

// ============================================================ What's new: the newest four, then "Show every version"
const changelog = document.querySelector(".changelog");
const moreBtn = document.querySelector("[data-more]");
if (changelog && moreBtn) {
  const older = Array.from(changelog.querySelectorAll(":scope > article")).slice(4);
  older.forEach((a) => a.classList.add("older"));
  if (older.length === 0) moreBtn.parentElement.hidden = true;
  moreBtn.textContent = `Show every version (${older.length} more)`;
  moreBtn.addEventListener("click", () => {
    const all = changelog.classList.toggle("all");
    moreBtn.setAttribute("aria-expanded", String(all));
    moreBtn.textContent = all ? "Show fewer" : `Show every version (${older.length} more)`;
    if (!all) document.getElementById("whats-new").scrollIntoView({ behavior: "smooth" });
  });
}

// ============================================================ The nav shows where you are
const navLinks = Array.from(document.querySelectorAll(".nav-links a"));
const navTargets = navLinks.map((a) => document.querySelector(a.getAttribute("href"))).filter(Boolean);
if ("IntersectionObserver" in window && navTargets.length) {
  const spy = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (!e.isIntersecting) continue;
      navLinks.forEach((a) => a.classList.toggle("on", a.getAttribute("href") === `#${e.target.id}`));
    }
  }, { rootMargin: "-45% 0px -50% 0px" });
  navTargets.forEach((t) => spy.observe(t));
}
