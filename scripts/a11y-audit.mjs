// Accessibility audit using axe-core. Runs in CI against the built web
// companion (which shares ~80% of its UI surface with the desktop app via
// the @slop/ui-timeline package).
//
// We require WCAG 2.2 AA. The build fails on any violation in that set.
//
// Run: pnpm --filter @slop/web test:a11y

import { chromium } from "playwright";
import AxeBuilder from "@axe-core/playwright";

const url = process.env.SLOP_AXE_URL ?? "http://localhost:5174";

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto(url, { waitUntil: "networkidle" });

const results = await new AxeBuilder({ page })
  .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa", "best-practice"])
  .disableRules([])
  .analyze();

if (results.violations.length > 0) {
  console.error("axe-core violations:");
  for (const v of results.violations) {
    console.error(`- [${v.impact}] ${v.id}: ${v.help} (${v.helpUrl})`);
    for (const node of v.nodes) {
      console.error(`    ${node.target.join(", ")}`);
    }
  }
  await browser.close();
  process.exit(1);
}

console.log(`axe-core: 0 violations (${results.passes.length} checks passed)`);
await browser.close();
