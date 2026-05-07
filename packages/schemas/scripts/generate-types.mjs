#!/usr/bin/env node
// Generate TypeScript types from the JSON Schema files.
// Run: pnpm --filter @slop/schemas codegen

import { compileFromFile } from "json-schema-to-typescript";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const outDir = join(root, "generated");
mkdirSync(outDir, { recursive: true });

const schemas = ["timeline.v1.json", "timeline.v2.json", "ops.v1.json", "plan.v1.json"];

for (const file of schemas) {
  const ts = await compileFromFile(join(root, file), {
    bannerComment:
      "/* eslint-disable */\n// AUTO-GENERATED from " + file + ". Do not edit by hand.\n",
    style: { semi: true, singleQuote: false },
  });
  const outName = file.replace(/\.json$/, ".ts");
  writeFileSync(join(outDir, outName), ts);
  console.log("wrote", join("generated", outName));
}

const indexLines = [];
for (const f of schemas) {
  const name = f.replace(/\.json$/, "");
  // Each schema gets its own namespace export so timeline.v1 and
  // timeline.v2 (which share interface names like SlopTimeline, Asset, ...)
  // don't collide. Also re-export the latest timeline schema's types at
  // top level for ergonomics.
  const safe = name.replace(/\./g, "_");
  indexLines.push(`import * as ${safe} from "./${name}";`);
  indexLines.push(`export { ${safe} };`);
}
// Default top-level types come from timeline.v1 for backwards compat
// with the V1 frontend; consumers wanting v2 types use `timeline_v2.*`.
indexLines.push(`export * from "./timeline.v1";`);
indexLines.push(`export * from "./ops.v1";`);
indexLines.push(`export * from "./plan.v1";`);
writeFileSync(join(outDir, "index.ts"), indexLines.join("\n") + "\n");
console.log("wrote generated/index.ts");
