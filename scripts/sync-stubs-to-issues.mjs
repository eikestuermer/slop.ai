#!/usr/bin/env node
// Sync the S-* punch-list rows in docs/stubs.md into GitHub issues so the
// "swarm of open-source developers" can find them in the issue tracker.
//
// Behaviour:
// - Parses every "### S-XXX-NNN — title" header in docs/stubs.md and the
//   bullets that follow until the next "###" or "##" header.
// - For each stub: ensures a GitHub issue exists with the stub id in the
//   title; updates the body to match the parsed row; applies labels
//   ["stub", "<workstream>", "effort:<S|M|L|XL>", "S-<area>"].
// - Stubs found under the "## Closed (was a stub, now real)" section close
//   their corresponding issue if it's still open.
//
// Modes:
//   --dry-run   parse + diff against the live tracker, print a plan, no writes
//   (default)   apply the plan
//
// Auth:
//   gh CLI must be authenticated with `repo` scope (`gh auth status`).
//
// Run from the repo root:
//   node scripts/sync-stubs-to-issues.mjs --dry-run
//   node scripts/sync-stubs-to-issues.mjs

import { readFile } from 'node:fs/promises';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import process from 'node:process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const execFileAsync = promisify(execFile);
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(__dirname, '..');
const STUBS_PATH = path.join(REPO_ROOT, 'docs', 'stubs.md');

const DRY_RUN = process.argv.includes('--dry-run');

const STUB_BODY_MARKER = '<!-- managed by scripts/sync-stubs-to-issues.mjs -->';

/**
 * @typedef Stub
 * @property {string} id            e.g. "S-RENDER-001"
 * @property {string} title         the part after the em-dash in the header
 * @property {'Engine'|'AI'|'Platform'|'Verification'|'Unknown'} workstream
 * @property {string|null} effort   "S" | "M" | "L" | "XL" | null
 * @property {string} body          full markdown body of the stub
 * @property {boolean} closed       true if it appeared under the "Closed" section
 */

async function parseStubs() {
  const raw = await readFile(STUBS_PATH, 'utf8');
  const lines = raw.split('\n');

  /** @type {Stub[]} */
  const stubs = [];
  let currentH2 = '';
  /** @type {Stub | null} */
  let current = null;
  /** @type {string[]} */
  let bodyLines = [];

  const flush = () => {
    if (!current) return;
    current.body = bodyLines.join('\n').trim();
    current.effort = extractEffort(current.body);
    stubs.push(current);
    current = null;
    bodyLines = [];
  };

  for (const line of lines) {
    if (line.startsWith('## ')) {
      flush();
      currentH2 = line.slice(3).trim();
      continue;
    }
    if (line.startsWith('### ')) {
      flush();
      const header = line.slice(4).trim();
      const m = header.match(/^(S-[A-Z]+-\d{3})\s*[—-]\s*(.+)$/);
      if (!m) continue;
      const [, id, title] = m;
      current = {
        id,
        title: title.trim(),
        workstream: workstreamFromH2(currentH2),
        effort: null,
        body: '',
        closed: currentH2.toLowerCase().startsWith('closed'),
      };
      continue;
    }
    if (current) bodyLines.push(line);
  }
  flush();
  return stubs;
}

/** @param {string} h2 @returns {Stub['workstream']} */
function workstreamFromH2(h2) {
  const lower = h2.toLowerCase();
  if (lower.startsWith('engine')) return 'Engine';
  if (lower === 'ai' || lower.startsWith('ai ')) return 'AI';
  if (lower.startsWith('platform')) return 'Platform';
  if (lower.startsWith('verification')) return 'Verification';
  return 'Unknown';
}

/** @param {string} body @returns {string|null} */
function extractEffort(body) {
  const m = body.match(/-\s*\*\*Effort\*\*:\s*(S|M|L|XL)\b/i);
  return m ? m[1].toUpperCase() : null;
}

/**
 * @param {Stub} stub
 * @returns {string}
 */
function renderIssueBody(stub) {
  return [
    STUB_BODY_MARKER,
    '',
    `Tracked in [\`docs/stubs.md\`](../blob/main/docs/stubs.md) as **${stub.id}**.`,
    '',
    'Auto-synced. To change the body, edit `docs/stubs.md` and re-run',
    '`node scripts/sync-stubs-to-issues.mjs`. Manual edits will be overwritten.',
    '',
    '---',
    '',
    stub.body,
    '',
    '---',
    '',
    `**To claim this stub**: open an issue using the [claim-stub template](../.github/ISSUE_TEMPLATE/claim-stub.yml) with id \`${stub.id}\`. A maintainer will assign you.`,
  ].join('\n');
}

/**
 * @param {Stub} stub
 * @returns {string[]}
 */
function labelsFor(stub) {
  const labels = ['stub'];
  if (stub.workstream !== 'Unknown') labels.push(`workstream:${stub.workstream.toLowerCase()}`);
  if (stub.effort) labels.push(`effort:${stub.effort}`);
  const areaMatch = stub.id.match(/^S-([A-Z]+)-\d{3}$/);
  if (areaMatch) labels.push(`area:${areaMatch[1].toLowerCase()}`);
  return labels;
}

/**
 * Find existing issues whose title starts with "[stub] S-..." so we can
 * idempotently update or close them.
 */
async function listExistingStubIssues() {
  const { stdout } = await execFileAsync('gh', [
    'issue', 'list',
    '--state', 'all',
    '--label', 'stub',
    '--limit', '500',
    '--json', 'number,title,state,labels,body',
  ]);
  /** @type {Array<{number:number,title:string,state:string,labels:Array<{name:string}>,body:string}>} */
  const all = JSON.parse(stdout);
  const byId = new Map();
  for (const issue of all) {
    const m = issue.title.match(/\b(S-[A-Z]+-\d{3})\b/);
    if (!m) continue;
    byId.set(m[1], issue);
  }
  return byId;
}

async function ghIssueCreate(title, body, labels) {
  const args = ['issue', 'create', '--title', title, '--body', body];
  for (const l of labels) args.push('--label', l);
  const { stdout } = await execFileAsync('gh', args);
  return stdout.trim();
}

async function ghIssueEdit(number, body, labels) {
  const args = ['issue', 'edit', String(number), '--body', body];
  for (const l of labels) args.push('--add-label', l);
  await execFileAsync('gh', args);
}

async function ghIssueClose(number) {
  await execFileAsync('gh', ['issue', 'close', String(number), '--reason', 'completed']);
}

async function main() {
  const stubs = await parseStubs();
  console.log(`Parsed ${stubs.length} stub rows from docs/stubs.md`);
  const closed = stubs.filter((s) => s.closed);
  const open = stubs.filter((s) => !s.closed);
  console.log(`  open: ${open.length}, closed: ${closed.length}`);

  if (DRY_RUN) {
    console.log('\n--- DRY RUN — no GitHub writes ---\n');
    for (const s of open) {
      console.log(`OPEN  ${s.id} [${s.workstream}, effort=${s.effort ?? '?'}] "${s.title}"`);
      console.log(`        labels: ${labelsFor(s).join(', ')}`);
    }
    for (const s of closed) {
      console.log(`CLOSE ${s.id} "${s.title}"`);
    }
    return;
  }

  if (!process.env.GITHUB_TOKEN && !(await hasGhAuth())) {
    console.error('Error: gh is not authenticated. Run `gh auth login` first.');
    process.exit(2);
  }

  const existing = await listExistingStubIssues();
  let created = 0;
  let updated = 0;
  let closedCount = 0;

  for (const s of open) {
    const title = `[stub] ${s.id} — ${s.title}`;
    const body = renderIssueBody(s);
    const labels = labelsFor(s);
    const found = existing.get(s.id);
    if (!found) {
      console.log(`creating: ${title}`);
      const url = await ghIssueCreate(title, body, labels);
      created++;
      console.log(`  -> ${url}`);
    } else {
      const sameBody = found.body && found.body.trim() === body.trim();
      const existingLabels = new Set((found.labels ?? []).map((l) => l.name));
      const labelsAlreadyApplied = labels.every((l) => existingLabels.has(l));
      if (sameBody && labelsAlreadyApplied && found.state === 'OPEN') {
        // nothing to do
        continue;
      }
      console.log(`updating: #${found.number} ${title}`);
      await ghIssueEdit(found.number, body, labels);
      updated++;
    }
  }

  for (const s of closed) {
    const found = existing.get(s.id);
    if (found && found.state === 'OPEN') {
      console.log(`closing: #${found.number} (${s.id})`);
      await ghIssueClose(found.number);
      closedCount++;
    }
  }

  console.log(`Done. created=${created} updated=${updated} closed=${closedCount}`);
}

async function hasGhAuth() {
  try {
    await execFileAsync('gh', ['auth', 'status']);
    return true;
  } catch {
    return false;
  }
}

main().catch((err) => {
  console.error(err.stack ?? err);
  process.exit(1);
});
