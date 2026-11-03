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

// ============================================================ Ban hammer animation
function triggerBanHammer() {
  const container = document.querySelector(".ban-hammer-container");
  const feathersContainer = container.querySelector(".feathers");

  // Clear previous feathers
  feathersContainer.innerHTML = "";

  // Create multiple feathers with random positions and rotations
  for (let i = 0; i < 20; i++) {
    const feather = document.createElement("div");
    feather.className = "feather";

    // Random starting position around the impact point
    const angle = (Math.PI * 2 * i) / 20;
    const radius = 50 + Math.random() * 50;
    const startX = 50 + Math.cos(angle) * radius;
    const startY = 50 + Math.sin(angle) * radius;

    feather.style.left = `${startX}%`;
    feather.style.top = `${startY}%`;
    feather.style.setProperty("--drift", `${(Math.random() - 0.5) * 200}px`);
    feather.style.setProperty("--rotate", `${Math.random() * 720 - 360}deg`);
    feather.style.animationDelay = `${1 + Math.random() * 0.3}s`;

    feathersContainer.appendChild(feather);
  }

  // Show the animation
  container.hidden = false;

  // Play impact sound (using Web Audio API to create a "thud" sound)
  playMaceSound();

  // Hide after animation completes
  setTimeout(() => {
    container.hidden = true;
  }, 4000);
}

function playMaceSound() {
  try {
    const audioContext = new (window.AudioContext || window.webkitAudioContext)();

    // Create a "thud" sound effect
    const duration = 0.5;
    const sampleRate = audioContext.sampleRate;
    const buffer = audioContext.createBuffer(1, sampleRate * duration, sampleRate);
    const data = buffer.getChannelData(0);

    // Generate a deep impact sound
    for (let i = 0; i < buffer.length; i++) {
      const t = i / sampleRate;
      // Combination of low frequencies for that heavy mace impact
      const freq1 = Math.sin(2 * Math.PI * 80 * t); // Low rumble
      const freq2 = Math.sin(2 * Math.PI * 150 * t); // Mid-low impact
      const noise = (Math.random() * 2 - 1) * 0.3; // Some noise for texture

      // Exponential decay for the impact fade
      const envelope = Math.exp(-t * 8);

      data[i] = (freq1 * 0.5 + freq2 * 0.3 + noise) * envelope * 0.3;
    }

    const source = audioContext.createBufferSource();
    source.buffer = buffer;
    source.connect(audioContext.destination);
    source.start();

  } catch (e) {
    console.log("Audio not available");
  }
}

// Easter egg: Press Shift+B+H together to trigger ban hammer
let keys = {};
document.addEventListener("keydown", (e) => {
  keys[e.key.toLowerCase()] = true;

  // Check for Shift + B + H
  if (e.shiftKey && keys["b"] && keys["h"]) {
    triggerBanHammer();
  }
});

document.addEventListener("keyup", (e) => {
  keys[e.key.toLowerCase()] = false;
});

// You can also trigger it programmatically:
// window.triggerBanHammer = triggerBanHammer;
