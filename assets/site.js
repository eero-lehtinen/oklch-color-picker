function preventBrowserShortcuts(event) {
  const isCtrl = event.ctrlKey || event.metaKey; // Ctrl on Windows/Linux, Cmd on Mac
  const isVimNavigation = isCtrl && ["h", "j", "k", "l"].includes(event.key);
  const isTabShortcut = ["F1", "F2"].includes(event.key);
  if (isVimNavigation || isTabShortcut) {
    event.preventDefault();
  }
}

// Keep application shortcuts from activating browser shortcuts.
document
  .getElementById("the_canvas_id")
  .addEventListener("keydown", preventBrowserShortcuts);

// Egui prevents the default wheel action on its canvas. Forward vertical
// wheel movement so users can continue scrolling to the page content.
document.getElementById("the_canvas_id").addEventListener(
  "wheel",
  (event) => {
    if (!event.ctrlKey && event.deltaY !== 0) {
      const scale =
        event.deltaMode === WheelEvent.DOM_DELTA_LINE
          ? 16
          : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
            ? window.innerHeight
            : 1;
      window.scrollBy(0, event.deltaY * scale);
    }
  },
  { passive: true },
);

// Do the same thing for the listener input created dynamically by egui.
const observer = new MutationObserver((mutationsList) => {
  for (const mutation of mutationsList) {
    for (const node of mutation.addedNodes) {
      if (node.nodeType === Node.ELEMENT_NODE && node.matches("input")) {
        node.addEventListener("keydown", preventBrowserShortcuts);
        observer.disconnect();
        return;
      }
    }
  }
});
observer.observe(document.body, {
  childList: true,
  subtree: false,
});

function scrollToCurrentHash() {
  const id = window.location.hash.slice(1);
  if (id) {
    document.getElementById(id)?.scrollIntoView({ block: "start" });
  }
}

function preserveInitialHashNavigation() {
  scrollToCurrentHash();

  const loadingText = document.getElementById("loading_text");
  if (!loadingText) {
    return;
  }

  const hashObserver = new MutationObserver(() => {
    if (!loadingText.isConnected) {
      hashObserver.disconnect();
      requestAnimationFrame(() => requestAnimationFrame(scrollToCurrentHash));
    }
  });
  hashObserver.observe(loadingText.parentNode, { childList: true });
}

window.addEventListener("load", preserveInitialHashNavigation);
window.addEventListener("hashchange", scrollToCurrentHash);

const scrollCue = document.querySelector(".scroll-cue");
let scrollCueUpdatePending = false;

function updateScrollCue() {
  const opacity = Math.max(0, 1 - window.scrollY / 240);
  scrollCue.style.opacity = opacity;
  scrollCue.style.pointerEvents = opacity === 0 ? "none" : "auto";
  scrollCueUpdatePending = false;
}

window.addEventListener(
  "scroll",
  () => {
    if (!scrollCueUpdatePending) {
      scrollCueUpdatePending = true;
      requestAnimationFrame(updateScrollCue);
    }
  },
  { passive: true },
);
updateScrollCue();

let zoomed = localStorage.getItem("zoomed") === "1";
if (zoomed) {
  zoomed = false;
  toggleZoom();
}

function toggleZoom() {
  const canvasClasses = document.querySelector("div.canvas-wrapper2").classList;
  const zoomButton = document.querySelector("button.fullscreen-button");
  zoomed = !zoomed;
  if (zoomed) {
    canvasClasses.remove("small");
  } else {
    canvasClasses.add("small");
  }
  zoomButton.setAttribute("aria-pressed", zoomed ? "true" : "false");
  localStorage.setItem("zoomed", zoomed ? "1" : "0");
}

// We disable caching during development so that we always view the latest version.
if ("serviceWorker" in navigator && window.location.hash !== "#dev") {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("sw.js");
  });
}
