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

// ============================================================ Preload critical images
const preloadImages = ["images/launcher-home.png", "images/ingame-menu.png"];
preloadImages.forEach(src => {
  const link = document.createElement("link");
  link.rel = "preload";
  link.as = "image";
  link.href = src;
  document.head.appendChild(link);
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
