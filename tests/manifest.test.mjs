import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const manifest = JSON.parse(fs.readFileSync(new URL("../package/planeai-plugin.json", import.meta.url), "utf8"));

test("manifest keeps the modal details contribution and release placeholder", () => {
  assert.equal(manifest.version, "0.0.0");
  const details = manifest.ui_contributions.find((contribution) => contribution.id === "details");
  assert.deepEqual(details, {
    id: "details",
    label: "Kiro Usage",
    placement: "session.panel",
    entrypoint: "ui/entry.js",
  });
  assert.equal(manifest.backend_entrypoints["macos-arm64"], "bin/macos-arm64/planeai-plugin-kiro-usage");
});
