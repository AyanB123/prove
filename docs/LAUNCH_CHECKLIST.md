# Launch checklist — Prove

Step-by-step public launch. Do not mark social posts done until you actually post. This file is the runbook.

Replace `AyanB123` with your GitHub username everywhere before go-live.

---

## 0. Preflight (local, required green)

From repo root:

```powershell
cargo build --release
cargo test
./target/release/prove doctor
./target/release/prove eval traps
# expect: false-done rate: naive 10/10 → prove 0/10
```

Split demo (pick one):

```powershell
# Windows
./demos/demo.ps1

# bash / Git Bash / WSL
./demos/demo.sh
```

Preflight pass criteria:

- [ ] `cargo test` green
- [ ] `prove doctor` green (or documents missing optional CLIs only)
- [ ] `prove eval traps` prints `naive 10/10 → prove 0/10`
- [ ] demo script shows naive blocked and local-loop done + `prove pr` bundle
- [ ] No secrets in terminal scrollback or `.prove/` samples you will screenshot
- [ ] LICENSE Apache-2.0 present; README install path works on clean clone mentally

---

## 1. Create public GitHub repo (you push)

1. Create empty public repo named `prove` under your account (no README/license if local already has them).
2. Confirm local git remote and branch:

```powershell
git status
git remote -v
# if needed:
# git remote add origin https://github.com/AyanB123/prove.git
```

3. Final local commit of launch docs if anything still dirty.
4. Push default branch:

```powershell
git push -u origin HEAD
```

5. GitHub UI checks:

- [ ] Repo is **public**
- [ ] About blurb: `Evidence-gated control plane for coding agents. Don't trust the agent. Trust the evidence.`
- [ ] Topics: `rust`, `cli`, `agents`, `developer-tools`, `llm`, `verification`
- [ ] LICENSE detected as Apache-2.0
- [ ] README renders metric + install + demo

You own the push. Do not claim launch complete until the clone URL works for strangers.

---

## 2. Replace URL placeholders

Search the tree for `AyanB123` and any `github.com/AyanB123/prove` stubs. Update at least:

| File | What to fix |
|------|-------------|
| `README.md` | `git clone https://github.com/AyanB123/prove.git` |
| `demos/SHOW_HN.md` | Repo URL in first comment + checklist |
| `demos/STORYBOARD-45s.md` | End card URL |
| `docs/job-packet.md` | Public repo line if still placeholder |
| `docs/LAUNCH_CHECKLIST.md` | This file after you know the real URL |
| `docs/APPLICATION_TARGETS.md` | Attach-repo lines if placeholder |

Optional: pin a release tag `v0.1.0` after push for a stable demo SHA.

Verify:

```powershell
rg -n "AyanB123|github.com/AyanB123" -g '!target/**'
```

Expect zero hits (or only historical notes you intentionally keep).

---

## 3. Record video from storyboard

Source of truth: `demos/STORYBOARD-45s.md`.

1. Pre-roll off camera: `cargo build --release`, large font, dark terminals titled NAIVE / PROVE.
2. Film shot list 0:00–0:45 (split naive vs local-loop, then full-width eval).
3. Hold FALSE-DONE ≥1.5s and DONE evidence matrix ≥2s.
4. End card: tagline + `false-done 10/10 → 0/10` + real GitHub URL (1s pause).
5. Export 1080p 30fps; optional 9:16 crop for short-form.

Checklist:

- [ ] `prove eval traps` line readable on camera
- [ ] URL on end card is the live public repo
- [ ] Upload unlisted first; switch to public at submit time
- [ ] Paste video link into Show HN first comment and social drafts

---

## 4. Show HN

Source: `demos/SHOW_HN.md`.

1. Title (chosen):  
   `Show HN: Prove – evidence-gated control plane for coding agents (false-done 10/10 → 0/10)`
2. Link: public GitHub repo (or landing that immediately shows repo + video).
3. Post US weekday morning if possible.
4. Within 2 minutes: paste first comment from `demos/SHOW_HN.md` with real URLs + video.
5. Stay in thread. Use reply bank in that file for CI / adapters / tamper / Rust questions.

Launch-day boxes:

- [ ] Public repo clone-tested
- [ ] Eval green on clean machine or fresh checkout
- [ ] Video public
- [ ] Placeholders gone
- [ ] First comment live
- [ ] Monitoring replies for 2–4 hours

---

## 5. Twitter / X and LinkedIn drafts

Fill `<url>` and `<video>` before posting. Keep metric and one command.

### Twitter / X (short)

```text
Agents can claim. Only evidence should advance.

I open-sourced Prove: a CLI control plane above Claude Code, Codex, Aider, and local loops.

Lifecycle moves only on git-bound receipts.
false-done: naive 10/10 → prove 0/10

cargo install path in README
<url>
<video>
```

Alternate hook:

```text
"All tests passed" is often a self-report.

Prove ignores backend claims and admits only policy commands on the live tree hash.

Trap suite: naive 10/10 false-done → prove 0/10

Don't trust the agent. Trust the evidence.
<url>
```

### LinkedIn (short professional)

```text
I shipped Prove, an open-source evidence-gated control plane for coding agents (Rust, Apache-2.0).

Problem: multi-CLI stacks share little memory and even less proof. Visible greens and chat logs still become "done."

Approach: backends propose; a pure lifecycle FSM commits only on git-state-bound receipts (head/tree + policy + command digests). prove pr refuses export without fresh admission.

Result on the public trap suite: false-done rate naive 10/10 → prove 0/10.

If you build coding agents, agent infra, or devtools, I would value feedback on the trust model and missing traps.

Repo: <url>
Demo: <video>
Write-up: docs/eval-report.md in the repo
```

Posting order suggestion: GitHub public → video public → Show HN → Twitter/X → LinkedIn (same day or next morning).

---

## 6. Apply-to-jobs sequence (use job packet)

Do this after the repo is public so links resolve.

1. Open `docs/job-packet.md` and `docs/APPLICATION_TARGETS.md`.
2. For each target:
   - Customize the 2-sentence pitch with one company-specific sentence (their product surface).
   - Attach or link: repo, 45s video, `docs/eval-report.md`, code tour paths.
   - Paste Email/LinkedIn blurb into outreach; keep under ~80 words.
   - Use one of the three discussion questions in the closer.
3. Track applications in a simple table (company, role, date, link, status).
4. Prefer roles that own agent runtime, harnesses, eval, or developer trust/UX over pure model-training posts unless you want research eng.

Minimum apply batch after launch:

- [ ] 5 priority targets from APPLICATION_TARGETS (P0)
- [ ] 5 secondary (P1)
- [ ] Warm intros where you have any path
- [ ] Pin job-packet + video in LinkedIn featured (optional)

---

## 7. Day-of launch order (compressed)

1. Preflight green  
2. Placeholder sweep  
3. Push public GitHub  
4. Publish video  
5. Show HN + first comment  
6. Twitter/X  
7. LinkedIn  
8. Start P0 applications with live links  

---

## 8. Explicitly not done by docs alone

These stay unchecked until humans perform them:

- [ ] Actual `git push` to public remote
- [ ] Recorded and published demo video
- [ ] Show HN submission
- [ ] Twitter/X post
- [ ] LinkedIn post
- [ ] Job applications sent

Docs and packaging can be complete while the boxes above remain open.

