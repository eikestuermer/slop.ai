#!/usr/bin/env node
// Validate that all schemas are syntactically valid JSON Schema 2020-12 and
// that they parse with ajv. Run in CI before any code that depends on them.

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats.default(ajv);

const schemas = ["timeline.v1.json", "ops.v1.json", "plan.v1.json"];
let failed = 0;
for (const file of schemas) {
  try {
    const schema = JSON.parse(readFileSync(join(root, file), "utf8"));
    ajv.compile(schema);
    console.log("ok", file);
  } catch (err) {
    console.error("FAIL", file, err.message);
    failed++;
  }
}
process.exit(failed ? 1 : 0);
