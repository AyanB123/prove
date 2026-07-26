import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";

type Receipt = {
  receipt_id?: string;
  mission_id?: string;
  claim_type?: string;
  head_hash?: string;
  tree_hash?: string;
  policy_hash?: string;
  command_set_hash?: string;
  touched_files_hash?: string;
  produced_at?: string;
  producer?: { backend?: string; run_id?: string };
  commands?: Array<{
    cmd?: string[];
    exit_code?: number;
    duration_ms?: number;
    stdout_excerpt?: string;
    stderr_excerpt?: string;
    stdout_sha256?: string;
    stderr_sha256?: string;
  }>;
  seal?: { alg?: string; key_id?: string; signature?: string };
};

export function openReceiptWebview(fsPath: string) {
  const panel = vscode.window.createWebviewPanel(
    "proveReceipt",
    `Receipt ${path.basename(fsPath, ".json")}`,
    vscode.ViewColumn.Beside,
    { enableScripts: false, retainContextWhenHidden: true }
  );
  panel.webview.html = renderReceiptHtml(fsPath);
}

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function shortHash(h?: string): string {
  if (!h) return "—";
  return h.length > 16 ? `${h.slice(0, 12)}…` : h;
}

function renderReceiptHtml(fsPath: string): string {
  let body = "";
  try {
    const raw = fs.readFileSync(fsPath, "utf8");
    const r = JSON.parse(raw) as Receipt;
    const cmds = (r.commands ?? [])
      .map((c, i) => {
        const ok = (c.exit_code ?? 1) === 0;
        const badge = ok ? "ok" : "fail";
        const cmd = esc((c.cmd ?? []).join(" "));
        const err = esc(c.stderr_excerpt || c.stdout_excerpt || "");
        return `<div class="cmd ${badge}">
          <div class="cmd-head"><span class="badge">${badge}</span> #${i + 1} exit ${c.exit_code ?? "?"} · ${c.duration_ms ?? 0}ms</div>
          <code>${cmd}</code>
          ${err ? `<pre>${err}</pre>` : ""}
        </div>`;
      })
      .join("");

    const seal = r.seal
      ? `<div class="row"><span>seal</span><b class="ok">${esc(r.seal.alg || "")} · ${esc(r.seal.key_id || "")}</b></div>
         <div class="row"><span>signature</span><code>${esc(shortHash(r.seal.signature))}</code></div>`
      : `<div class="row"><span>seal</span><b class="muted">none</b></div>`;

    body = `
      <div class="hero">
        <div class="claim">${esc(r.claim_type || "receipt")}</div>
        <div class="id">${esc(r.receipt_id || path.basename(fsPath))}</div>
      </div>
      <div class="grid">
        <div class="row"><span>mission</span><code>${esc(r.mission_id || "—")}</code></div>
        <div class="row"><span>producer</span><code>${esc(r.producer?.backend || "—")} / ${esc(r.producer?.run_id || "")}</code></div>
        <div class="row"><span>produced</span><code>${esc(r.produced_at || "—")}</code></div>
        <div class="row"><span>head</span><code>${esc(shortHash(r.head_hash))}</code></div>
        <div class="row"><span>tree</span><code>${esc(shortHash(r.tree_hash))}</code></div>
        <div class="row"><span>policy</span><code>${esc(shortHash(r.policy_hash))}</code></div>
        <div class="row"><span>commands</span><code>${esc(shortHash(r.command_set_hash))}</code></div>
        ${seal}
      </div>
      <h2>Commands (evidence)</h2>
      ${cmds || "<p class='muted'>No commands on this receipt.</p>"}
      <p class="foot">Agents can claim. Only evidence can advance.</p>
    `;
  } catch (e) {
    body = `<p class="fail">Failed to parse receipt: ${esc(String(e))}</p><pre>${esc(
      fs.existsSync(fsPath) ? fs.readFileSync(fsPath, "utf8").slice(0, 4000) : ""
    )}</pre>`;
  }

  return `<!DOCTYPE html>
<html>
<head>
<meta charset="UTF-8" />
<style>
  :root { color-scheme: light dark; }
  body { font-family: var(--vscode-font-family); padding: 16px 20px; color: var(--vscode-foreground); }
  .hero { margin-bottom: 16px; }
  .claim { font-size: 12px; text-transform: uppercase; letter-spacing: .08em; opacity: .7; }
  .id { font-size: 18px; font-weight: 600; word-break: break-all; }
  .grid { display: grid; gap: 8px; margin: 12px 0 20px; }
  .row { display: grid; grid-template-columns: 110px 1fr; gap: 8px; align-items: start; }
  .row span { opacity: .65; font-size: 12px; }
  code, pre { font-family: var(--vscode-editor-font-family, ui-monospace, monospace); font-size: 12px; }
  code { word-break: break-all; }
  pre { white-space: pre-wrap; background: color-mix(in srgb, var(--vscode-editor-background) 80%, #888 20%); padding: 8px; border-radius: 6px; }
  .cmd { border: 1px solid color-mix(in srgb, var(--vscode-foreground) 18%, transparent); border-radius: 8px; padding: 10px; margin: 8px 0; }
  .cmd.ok { border-color: color-mix(in srgb, #3dd68c 50%, transparent); }
  .cmd.fail { border-color: color-mix(in srgb, #f85149 50%, transparent); }
  .cmd-head { font-size: 12px; margin-bottom: 6px; opacity: .85; }
  .badge { display: inline-block; padding: 1px 6px; border-radius: 999px; margin-right: 6px; font-weight: 600; }
  .ok .badge { background: color-mix(in srgb, #3dd68c 25%, transparent); }
  .fail .badge { background: color-mix(in srgb, #f85149 25%, transparent); }
  .ok { color: #3dd68c; }
  .fail { color: #f85149; }
  .muted { opacity: .6; }
  .foot { margin-top: 24px; opacity: .55; font-size: 12px; }
  h2 { font-size: 14px; margin-top: 18px; }
</style>
</head>
<body>${body}</body>
</html>`;
}
