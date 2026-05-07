# integration-bindings-node

Closes `S-NODE-001`. Builds the napi-rs `.node` artifact, requires it from Node, smoke-tests.

## Job body (already in `ci.yml`)

```yaml
- uses: pnpm/action-setup@v4
- run: pnpm install --frozen-lockfile
- name: Build + smoke
  working-directory: bindings/node
  run: |
    pnpm build
    node -e "const m = require('./'); const t = new m.JsTimeline(); console.log(t.toJson());"
```

## Phase B extension

Add a richer smoke:

```js
const { JsTimeline } = require("./");
const fs = require("fs");
const path = require("path");
const os = require("os");
const t = new JsTimeline();
console.assert(t.nTracks() === 0);
const out = path.join(os.tmpdir(), "empty.otio");
t.exportOtio(out);
const doc = JSON.parse(fs.readFileSync(out, "utf8"));
console.assert(doc.OTIO_SCHEMA === "Timeline.1");
```

## Promotion criteria

- `.node` artifact builds on macOS + Linux.
- Smoke require + export works.
- Then promote, mark `S-NODE-001` green.
