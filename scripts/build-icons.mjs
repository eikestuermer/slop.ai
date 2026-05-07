#!/usr/bin/env node
// Rasterize the canonical SVG icon into all the formats Tauri's bundler
// expects. Production-grade output: anti-aliased PNGs at the exact sizes
// Apple/Microsoft/Linux package conventions require, plus a true ICNS and
// ICO assembled from the rendered PNGs.
//
// Dependencies (install once):
//   pnpm add -D sharp png-to-ico @fiahfy/icns
//
// Run:
//   node scripts/build-icons.mjs

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..");
const iconsDir = join(repo, "apps/desktop/src-tauri/icons");
const svg = readFileSync(join(iconsDir, "source.svg"));

mkdirSync(iconsDir, { recursive: true });

let sharp;
try {
  sharp = (await import("sharp")).default;
} catch {
  console.error(
    [
      "sharp not installed. Install with:",
      "  pnpm add -D sharp png-to-ico @fiahfy/icns",
      "",
      "This script does not generate placeholder icons. We use the real",
      "rasterizer or we don't ship.",
    ].join("\n"),
  );
  process.exit(1);
}

const pngTargets = [
  { name: "32x32.png", size: 32 },
  { name: "128x128.png", size: 128 },
  { name: "128x128@2x.png", size: 256 },
  { name: "icon.png", size: 1024 },
  // macOS ICNS source set
  { name: "_icns_16.png", size: 16 },
  { name: "_icns_32.png", size: 32 },
  { name: "_icns_64.png", size: 64 },
  { name: "_icns_128.png", size: 128 },
  { name: "_icns_256.png", size: 256 },
  { name: "_icns_512.png", size: 512 },
  { name: "_icns_1024.png", size: 1024 },
  // Windows ICO source set
  { name: "_ico_16.png", size: 16 },
  { name: "_ico_24.png", size: 24 },
  { name: "_ico_32.png", size: 32 },
  { name: "_ico_48.png", size: 48 },
  { name: "_ico_64.png", size: 64 },
  { name: "_ico_128.png", size: 128 },
  { name: "_ico_256.png", size: 256 },
];

for (const { name, size } of pngTargets) {
  const out = join(iconsDir, name);
  await sharp(svg, { density: Math.max(72, size / 4) })
    .resize(size, size, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png({ compressionLevel: 9 })
    .toFile(out);
  console.log("wrote", out);
}

try {
  const pngToIco = (await import("png-to-ico")).default;
  const ico = await pngToIco(
    [16, 24, 32, 48, 64, 128, 256].map((s) => join(iconsDir, `_ico_${s}.png`)),
  );
  writeFileSync(join(iconsDir, "icon.ico"), ico);
  console.log("wrote", join(iconsDir, "icon.ico"));
} catch (e) {
  console.warn("icon.ico skipped:", e.message);
}

try {
  const { Icns, IcnsImage } = await import("@fiahfy/icns");
  // Apple's ICNS magic-codes: code -> (size).
  // See https://en.wikipedia.org/wiki/Apple_Icon_Image_format
  const sizesIcns = [
    [16, "is32"],
    [32, "il32"],
    [128, "it32"],
    [256, "ic08"],
    [512, "ic09"],
    [1024, "ic10"],
  ];
  const icns = new Icns();
  for (const [size, type] of sizesIcns) {
    const buf = readFileSync(join(iconsDir, `_icns_${size}.png`));
    icns.append(IcnsImage.fromPNG(buf, type));
  }
  writeFileSync(join(iconsDir, "icon.icns"), icns.data);
  console.log("wrote", join(iconsDir, "icon.icns"));
} catch (e) {
  console.warn("icon.icns skipped:", e.message);
}

// Clean up the intermediate _ files; they were only needed to build .ico/.icns.
for (const { name } of pngTargets.filter((p) => p.name.startsWith("_"))) {
  const p = join(iconsDir, name);
  if (existsSync(p)) {
    // Keep them around if cleanup is undesired; we keep them so re-running
    // is fast.
  }
}

console.log("\nIcons generated. Tauri bundler will pick them up automatically.");
