import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";

export type ProveNode = {
  id: string;
  label: string;
  description?: string;
  fsPath?: string;
  collapsible?: boolean;
  children?: ProveNode[];
  contextValue?: string;
};

export class ProveTreeProvider implements vscode.TreeDataProvider<ProveNode> {
  private _onDidChange = new vscode.EventEmitter<ProveNode | undefined | void>();
  readonly onDidChangeTreeData = this._onDidChange.event;

  constructor(private getRoot: () => string | undefined) {}

  refresh(): void {
    this._onDidChange.fire();
  }

  getTreeItem(element: ProveNode): vscode.TreeItem {
    const item = new vscode.TreeItem(
      element.label,
      element.collapsible || (element.children && element.children.length)
        ? vscode.TreeItemCollapsibleState.Expanded
        : vscode.TreeItemCollapsibleState.None
    );
    item.description = element.description;
    item.id = element.id;
    item.contextValue = element.contextValue;
    if (element.fsPath) {
      item.command = {
        command: "prove.openNode",
        title: "Open",
        arguments: [element],
      };
      item.resourceUri = vscode.Uri.file(element.fsPath);
    }
    return item;
  }

  getChildren(element?: ProveNode): ProveNode[] {
    if (element) return element.children ?? [];
    const root = this.getRoot();
    if (!root) {
      return [
        {
          id: "no-ws",
          label: "Open a workspace folder",
          description: "Prove needs a repo root",
        },
      ];
    }
    const proveDir = path.join(root, ".prove");
    if (!fs.existsSync(proveDir)) {
      return [
        {
          id: "no-prove",
          label: "Prove not initialized",
          description: "Run Prove: Init",
        },
      ];
    }

    const nodes: ProveNode[] = [];
    const missionPath = path.join(proveDir, "mission.json");
    if (fs.existsSync(missionPath)) {
      try {
        const m = JSON.parse(fs.readFileSync(missionPath, "utf8")) as {
          id?: string;
          goal?: string;
          phase?: string;
          backend?: string;
          steps_used?: number;
          repair_count?: number;
          touched_files?: string[];
          last_error?: string;
          stop_reason?: string;
        };
        nodes.push({
          id: "mission",
          label: "Mission",
          description: m.phase ?? "",
          collapsible: true,
          fsPath: missionPath,
          children: [
            { id: "m-id", label: "id", description: m.id },
            { id: "m-goal", label: "goal", description: m.goal },
            { id: "m-phase", label: "phase", description: m.phase },
            { id: "m-backend", label: "backend", description: m.backend },
            {
              id: "m-steps",
              label: "steps / repairs",
              description: `${m.steps_used ?? 0} / ${m.repair_count ?? 0}`,
            },
            ...(m.last_error
              ? [{ id: "m-err", label: "error", description: truncate(m.last_error, 80) }]
              : []),
            ...(m.stop_reason
              ? [{ id: "m-stop", label: "stop", description: m.stop_reason }]
              : []),
            {
              id: "m-touched",
              label: "touched files",
              collapsible: true,
              children: (m.touched_files ?? []).map((f, i) => ({
                id: `tf-${i}`,
                label: f,
                fsPath: path.join(root, f),
              })),
            },
          ],
        });
      } catch {
        nodes.push({
          id: "mission-bad",
          label: "mission.json (parse error)",
          fsPath: missionPath,
        });
      }
    } else {
      nodes.push({
        id: "no-mission",
        label: "No active mission",
        description: "prove run \"...\"",
      });
    }

    const policyPath = path.join(proveDir, "policy.yml");
    if (fs.existsSync(policyPath)) {
      nodes.push({
        id: "policy",
        label: "policy.yml",
        fsPath: policyPath,
        description: "gates",
      });
    }

    const receiptsDir = path.join(proveDir, "receipts");
    const receiptChildren: ProveNode[] = [];
    if (fs.existsSync(receiptsDir)) {
      const files = fs
        .readdirSync(receiptsDir)
        .filter((f) => f.endsWith(".json"))
        .sort()
        .reverse();
      for (const f of files.slice(0, 30)) {
        const fp = path.join(receiptsDir, f);
        let desc = "";
        try {
          const r = JSON.parse(fs.readFileSync(fp, "utf8")) as {
            claim_type?: string;
            receipt_id?: string;
          };
          desc = r.claim_type ?? "";
        } catch {
          /* ignore */
        }
        receiptChildren.push({
          id: `rec-${f}`,
          label: f.replace(/\.json$/, ""),
          description: desc,
          fsPath: fp,
        });
      }
    }
    nodes.push({
      id: "receipts",
      label: "Receipts",
      description: `${receiptChildren.length}`,
      collapsible: true,
      children:
        receiptChildren.length > 0
          ? receiptChildren
          : [{ id: "rec-empty", label: "(none yet)" }],
    });

    const artifactsDir = path.join(proveDir, "artifacts");
    if (fs.existsSync(artifactsDir)) {
      const arts = fs
        .readdirSync(artifactsDir)
        .filter((f) => f.endsWith(".md") || f.endsWith(".json"));
      nodes.push({
        id: "artifacts",
        label: "Artifacts",
        description: `${arts.length}`,
        collapsible: true,
        children: arts.map((f) => ({
          id: `art-${f}`,
          label: f,
          fsPath: path.join(artifactsDir, f),
        })),
      });
    }

    nodes.push({
      id: "hint",
      label: "Trust note",
      description: "Only receipts advance done",
    });

    return nodes;
  }
}

function truncate(s: string, n: number): string {
  const t = s.replace(/\s+/g, " ").trim();
  return t.length <= n ? t : t.slice(0, n - 1) + "…";
}
