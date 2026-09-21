import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourcePath = path.join(root, "package/ui/entry.js");
const iconPath = path.join(root, "package/ui/assets/kiro-icon.svg");
const outputPath = path.join(root, "build/ui/entry.js");
const placeholder = '"__KIRO_ICON_SVG__"';
const source = fs.readFileSync(sourcePath, "utf8");
const matches = source.split(placeholder).length - 1;
if (matches !== 1) {
  throw new Error(`expected one Kiro icon placeholder in ${sourcePath}, found ${matches}`);
}

const icon = fs
  .readFileSync(iconPath, "utf8")
  .trim()
  .replace(/^<svg\s/, '<svg class="kiro-icon" aria-hidden="true" focusable="false" ');
fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, source.replace(placeholder, JSON.stringify(icon)));
console.log(`Built ${path.relative(root, outputPath)} with bundled Kiro icon`);
