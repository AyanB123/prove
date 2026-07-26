import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import { spawn } from "child_process";
import { ProveTreeProvider, ProveNode } from "./proveTree";
import { openReceiptWebview } from "./receiptWebview";

let statusBar: vscode.StatusBarItem;
let tree: ProveTreeProvider;
let watcher: vscode.FileSystemWatcher | undefined;
let refreshTimer: ReturnType<typeof setInterval> | undefined;

export function activate(context: vscode.ExtensionContext) {
  tree = new ProveTreeProvider(() => getWorkspaceRoot());
  context.subscriptions.push(
    vscode.window.registerTreeDataProvider("prove.mission", tree)
  );

  statusBar = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100
  );
  statusBar.command = "prove.status";
  context.subscriptions.push(statusBar);

  context.subscriptions.push(
    vscode.commands.registerCommand("prove.refresh", () => {
      void refreshAll();
    }),
    vscode.commands.registerCommand("prove.status", () => {
      void runCli(["status"], true);
    }),
    vscode.commands.registerCommand("prove.verify", () => {
      void runCli(["verify"], true);
    }),
    vscode.commands.registerCommand("prove.verifyCi", () => {
      void runCli(["verify", "--ci", "--json"], true);
    }),
    vscode.commands.registerCommand("prove.doctor", () => {
      void runCli(["doctor"], true);
    }),
    vscode.commands.registerCommand("prove.init", () => {
      void runCli(["init"], true);
    }),
    vscode.commands.registerCommand("prove.openPolicy", () => {
      void openRel(".prove/policy.yml");
    }),
    vscode.commands.registerCommand("prove.openMission", () => {
      void openRel(".prove/mission.json");
    }),
    vscode.commands.registerCommand("prove.openNode", async (node: ProveNode) => {
      if (!node?.fsPath) return;
      const isReceipt =
        node.fsPath.replace(/\\/g, "/").includes("/.prove/receipts/") &&
        node.fsPath.endsWith(".json");
      if (isReceipt) {
        openReceiptWebview(node.fsPath);
        return;
      }
      const doc = await vscode.workspace.openTextDocument(node.fsPath);
      await vscode.window.showTextDocument(doc, { preview: true });
    }),
    vscode.commands.registerCommand("prove.openReceipt", (fsPath: string) => {
      if (fsPath) openReceiptWebview(fsPath);
    })
  );

  setupWatcher(context);
  void refreshAll();

  const ms = vscode.workspace
    .getConfiguration("prove")
    .get<number>("autoRefreshMs", 3000);
  if (ms > 0) {
    refreshTimer = setInterval(() => {
      void refreshAll();
    }, ms);
  }
}

export function deactivate() {
  if (refreshTimer) clearInterval(refreshTimer);
  watcher?.dispose();
}

function getWorkspaceRoot(): string | undefined {
  return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

function setupWatcher(context: vscode.ExtensionContext) {
  watcher = vscode.workspace.createFileSystemWatcher("**/.prove/**");
  const bump = () => {
    void refreshAll();
  };
  watcher.onDidChange(bump);
  watcher.onDidCreate(bump);
  watcher.onDidDelete(bump);
  context.subscriptions.push(watcher);
}

async function refreshAll() {
  tree.refresh();
  const root = getWorkspaceRoot();
  if (!root) {
    statusBar.hide();
    return;
  }
  const missionPath = path.join(root, ".prove", "mission.json");
  if (!fs.existsSync(missionPath)) {
    statusBar.text = "$(shield) Prove";
    statusBar.tooltip = "No active mission — run Prove: Init / prove run";
    statusBar.show();
    return;
  }
  try {
    const raw = fs.readFileSync(missionPath, "utf8");
    const m = JSON.parse(raw) as { phase?: string; goal?: string; id?: string };
    const phase = m.phase ?? "unknown";
    const icon =
      phase === "done" || phase === "pr_ready"
        ? "$(check)"
        : phase === "failed"
          ? "$(error)"
          : "$(shield)";
    statusBar.text = `${icon} Prove: ${phase}`;
    statusBar.tooltip = `${m.goal ?? ""}\n${m.id ?? ""}`.trim();
    statusBar.show();
  } catch {
    statusBar.text = "$(shield) Prove";
    statusBar.show();
  }
}

function cliPath(): string {
  return (
    vscode.workspace.getConfiguration("prove").get<string>("cliPath") || "prove"
  );
}

async function runCli(args: string[], showOutput: boolean): Promise<number> {
  const root = getWorkspaceRoot();
  if (!root) {
    vscode.window.showErrorMessage("Prove: open a workspace folder first.");
    return 1;
  }
  const out = vscode.window.createOutputChannel("Prove", { log: true });
  if (showOutput) out.show(true);
  out.appendLine(`$ ${cliPath()} ${args.join(" ")}`);

  return await new Promise((resolve) => {
    const child = spawn(cliPath(), args, {
      cwd: root,
      shell: process.platform === "win32",
      env: process.env,
    });
    child.stdout.on("data", (d: Buffer) => out.append(d.toString()));
    child.stderr.on("data", (d: Buffer) => out.append(d.toString()));
    child.on("error", (err) => {
      out.appendLine(`error: ${err.message}`);
      out.appendLine(
        "Is the prove CLI installed? cargo install --path . from the prove repo, or set prove.cliPath"
      );
      vscode.window.showErrorMessage(`Prove CLI failed: ${err.message}`);
      resolve(1);
    });
    child.on("close", (code) => {
      out.appendLine("");
      out.appendLine(`[exit ${code ?? 0}]`);
      void refreshAll();
      resolve(code ?? 0);
    });
  });
}

async function openRel(rel: string) {
  const root = getWorkspaceRoot();
  if (!root) return;
  const full = path.join(root, rel);
  if (!fs.existsSync(full)) {
    vscode.window.showWarningMessage(
      `Missing ${rel}. Run Prove: Init or prove run.`
    );
    return;
  }
  const doc = await vscode.workspace.openTextDocument(full);
  await vscode.window.showTextDocument(doc);
}

