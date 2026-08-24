import path from "node:path";
import { generateSW } from "workbox-build";

const stagingDirectory = process.env.TRUNK_STAGING_DIR;
const outputDirectory =
  process.platform === "win32"
    ? (stagingDirectory?.replace(/^\\\\\?\\/, "") ?? "dist")
    : (stagingDirectory ?? "dist");

const { count, size, warnings } = await generateSW({
  cacheId: "oklch-color-picker",
  cleanupOutdatedCaches: true,
  clientsClaim: true,
  directoryIndex: "index.html",
  dontCacheBustURLsMatching: /-[0-9a-f]{16}(?:_bg)?\.(?:css|js|wasm)$/,
  globDirectory: outputDirectory,
  globIgnores: ["sw.js"],
  globPatterns: [
    "index.html",
    "*.{css,js,wasm}",
    "assets/*.svg",
    "icon-192.png",
    "icon-512.png",
    "apple-touch-icon.png",
    "manifest.json",
  ],
  inlineWorkboxRuntime: true,
  maximumFileSizeToCacheInBytes: Number.MAX_SAFE_INTEGER,
  mode: "production",
  skipWaiting: true,
  sourcemap: false,
  swDest: path.join(outputDirectory, "sw.js"),
});

if (warnings.length > 0) {
  throw new Error(
    `Failed to generate the service worker:\n${warnings.join("\n")}`,
  );
}

console.log(
  `Generated service worker precaching ${count} files (${size} bytes).`,
);
