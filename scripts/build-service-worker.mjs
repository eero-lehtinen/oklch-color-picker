import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const stagingDirectory = process.env.TRUNK_STAGING_DIR;
const outputDirectory =
  process.platform === "win32"
    ? (stagingDirectory?.replace(/^\\\\\?\\/, "") ?? "dist")
    : (stagingDirectory ?? "dist");

const precachedExtensions = new Set([
  ".css",
  ".html",
  ".ico",
  ".js",
  ".json",
  ".png",
  ".svg",
  ".wasm",
]);

async function findPrecachedResources(directory, relativeDirectory = "") {
  const resources = [];
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries) {
    const relativePath = path.join(relativeDirectory, entry.name);
    const absolutePath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      resources.push(
        ...(await findPrecachedResources(absolutePath, relativePath)),
      );
    } else if (
      relativePath !== "sw.js" &&
      precachedExtensions.has(path.extname(entry.name).toLowerCase())
    ) {
      resources.push(relativePath.split(path.sep).join("/"));
    }
  }
  return resources;
}

const precachedResources = (
  await findPrecachedResources(outputDirectory)
).sort();

const contentHash = createHash("sha256");
let totalBytes = 0;
for (const resource of precachedResources) {
  const contents = await readFile(path.join(outputDirectory, resource));
  contentHash.update(resource);
  contentHash.update(contents);
  totalBytes += contents.byteLength;
}
const cacheName = `oklch-color-picker-${contentHash.digest("hex").slice(0, 16)}`;

const templatePath = path.join("assets", "sw.js");
const template = await readFile(templatePath, "utf8");
const cacheNamePlaceholder =
  'const cacheName = "oklch-color-picker-development";';
const resourcesPlaceholder = "const precachedResources = [];";
if (
  !template.includes(cacheNamePlaceholder) ||
  !template.includes(resourcesPlaceholder)
) {
  throw new Error(
    `Service worker placeholders are missing from ${templatePath}.`,
  );
}
const serviceWorker = template
  .replace(
    cacheNamePlaceholder,
    `const cacheName = ${JSON.stringify(cacheName)};`,
  )
  .replace(
    resourcesPlaceholder,
    `const precachedResources = ${JSON.stringify(precachedResources)};`,
  );

await writeFile(path.join(outputDirectory, "sw.js"), serviceWorker);
console.log(
  `Generated service worker precaching ${precachedResources.length} files (${totalBytes} bytes).`,
);
