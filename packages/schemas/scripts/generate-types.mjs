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

const schemas = ["timeline.v1.json", "ops.v1.json", "plan.v1.json"];

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

const indexLines = schemas.map((f) => {
  const name = f.replace(/\.json$/, "");
  return `export * from "./${name}";`;
});
writeFileSync(join(outDir, "index.ts"), indexLines.join("\n") + "\n");
console.log("wrote generated/index.ts");
