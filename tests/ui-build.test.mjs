import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

const source = fs.readFileSync(new URL("../package/ui/entry.js", import.meta.url), "utf8");
const icon = fs.readFileSync(new URL("../package/ui/assets/kiro-icon.svg", import.meta.url), "utf8");
const built = fs.readFileSync(new URL("../build/ui/entry.js", import.meta.url), "utf8");

test("build inlines the bundled Kiro ghost icon as decorative titlebar markup", () => {
  assert.match(icon, /^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" width="256" height="256" viewBox="0 0 256 256">/);
  assert.match(source, /const KIRO_ICON_SVG = "__KIRO_ICON_SVG__";/);
  assert.doesNotMatch(built, /__KIRO_ICON_SVG__/);
  assert.match(built, /class=\\"kiro-icon\\" aria-hidden=\\"true\\" focusable=\\"false\\"/);
  assert.match(built, /<span class="usage" data-usage>—<\/span>/);
});
