/**
 * Minimal VSIX packager (no vsce dependency).
 * VSIX = zip with extension files at archive root.
 */
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const extRoot = path.resolve(__dirname, "..", "extensions", "vscode-prove");
const pkg = JSON.parse(fs.readFileSync(path.join(extRoot, "package.json"), "utf8"));
const outName = `prove-${pkg.version}.vsix`;
const outPath = path.join(extRoot, outName);
const stage = path.join(require("os").tmpdir(), `prove-vsix-${pkg.version}-${process.pid}`);

function rm(p) {
  fs.rmSync(p, { recursive: true, force: true });
}
function copy(src, dest) {
  const st = fs.statSync(src);
  if (st.isDirectory()) {
    fs.mkdirSync(dest, { recursive: true });
    for (const name of fs.readdirSync(src)) {
      if (name === "node_modules" || name === "src" || name.endsWith(".vsix") || name.endsWith(".map")) continue;
      copy(path.join(src, name), path.join(dest, name));
    }
  } else {
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.copyFileSync(src, dest);
  }
}

rm(stage);
fs.mkdirSync(stage, { recursive: true });
// include only publishable
for (const name of ["package.json", "README.md", "CHANGELOG.md", "LICENSE", "media", "out"]) {
  const s = path.join(extRoot, name);
  if (fs.existsSync(s)) copy(s, path.join(stage, name));
}
// strip source maps from out
const outDir = path.join(stage, "out");
if (fs.existsSync(outDir)) {
  for (const f of fs.readdirSync(outDir)) {
    if (f.endsWith(".map")) fs.unlinkSync(path.join(outDir, f));
  }
}

if (fs.existsSync(outPath)) fs.unlinkSync(outPath);

// Prefer PowerShell Compress-Archive on Windows; zip on unix
if (process.platform === "win32") {
  const ps = `Compress-Archive -Path '${stage.replace(/'/g, "''")}\\*' -DestinationPath '${outPath.replace(/'/g, "''").replace(/\.vsix$/, ".zip")}' -Force`;
  execFileSync("powershell.exe", ["-NoProfile", "-Command", ps], { stdio: "inherit" });
  const zipPath = outPath.replace(/\.vsix$/, ".zip");
  fs.renameSync(zipPath, outPath);
} else {
  execFileSync("zip", ["-r", outPath, "."], { cwd: stage, stdio: "inherit" });
}

rm(stage);
const size = fs.statSync(outPath).size;
console.log(`wrote ${outPath} (${size} bytes)`);
