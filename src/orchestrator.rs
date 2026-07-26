use crate::adapters::{self, TurnInput};
use crate::git_state;
use crate::lifecycle::{self, Phase, TransitionEvent};
use crate::memory::{FileLocks, MissionMemory};
use crate::policy::Policy;
use crate::receipts::{
    self, ClaimType, Producer, ReceiptStore,
};
use crate::store::{Mission, ProveStore};
use anyhow::{anyhow, bail, Result};
use colored::Colorize;
use uuid::Uuid;

pub struct Orchestrator<'a> {
    pub store: &'a ProveStore,
    pub policy: Policy,
}

impl<'a> Orchestrator<'a> {
    pub fn new(store: &'a ProveStore) -> Result<Self> {
        store.require_initialized()?;
        let policy = store.load_policy()?;
        Ok(Self { store, policy })
    }

    pub fn run_mission(&self, goal: &str, backend: Option<&str>) -> Result<Mission> {
        let backend_id = adapters::route_backend(goal, backend);
        let mut mission = Mission::new(goal, &backend_id);
        self.store.save_mission(&mission)?;
        self.store
            .append_event("mission_start", &format!("{} via {backend_id}", goal))?;

        let memory = MissionMemory::open(&self.store.prove_dir)?;
        memory.append(
            "goal",
            goal,
            serde_json::json!({"backend": backend_id}),
        )?;

        // Plan admitted (local deterministic plan for v1)
        let tr = lifecycle::apply(mission.phase, TransitionEvent::PlanOk);
        mission.phase = tr.to;
        mission.updated_at = chrono::Utc::now();
        mission.steps_used += 1;
        self.store.save_mission(&mission)?;
        println!("{} {}", "→".cyan(), tr.reason);

        self.loop_until_terminal(&mut mission, &memory)?;
        Ok(mission)
    }

    pub fn resume(&self) -> Result<Mission> {
        let mut mission = self.store.load_mission()?;
        let memory = MissionMemory::open(&self.store.prove_dir)?;
        self.loop_until_terminal(&mut mission, &memory)?;
        Ok(mission)
    }

    fn loop_until_terminal(&self, mission: &mut Mission, memory: &MissionMemory) -> Result<()> {
        let locks = FileLocks::open(&self.store.prove_dir)?;
        let receipts = ReceiptStore::open(&self.store.prove_dir)?;

        while !matches!(
            mission.phase,
            Phase::Done | Phase::PrReady | Phase::Failed
        ) {
            if mission.steps_used >= self.policy.budgets.max_steps {
                self.stop(mission, "max_steps budget exceeded")?;
                break;
            }
            if mission.repair_count > self.policy.gates.test.repair_limit {
                self.stop(
                    mission,
                    &format!(
                        "repair_limit {} exceeded",
                        self.policy.gates.test.repair_limit
                    ),
                )?;
                break;
            }

            match mission.phase {
                Phase::Patching => self.do_patch(mission, memory, &locks, &receipts)?,
                Phase::Testing => self.do_test(mission, memory, &receipts)?,
                Phase::Reviewing => self.do_review(mission, memory, &receipts)?,
                other => {
                    bail!(
                        "internal error: unexpected phase {:?} in mission loop\n  \
                         Fix: inspect .prove/mission.json or start a new mission with `prove run`",
                        other
                    );
                }
            }
            self.store.save_mission(mission)?;
        }
        let _ = locks.release_all(&mission.id);
        self.store.save_mission(mission)?;
        self.print_status(mission)?;
        Ok(())
    }

    fn do_patch(
        &self,
        mission: &mut Mission,
        memory: &MissionMemory,
        locks: &FileLocks,
        receipts: &ReceiptStore,
    ) -> Result<()> {
        println!(
            "{} patching with backend {}",
            "●".yellow(),
            mission.backend.cyan()
        );
        let adapter = adapters::get_adapter(&mission.backend)?;
        let input = TurnInput {
            mission_id: mission.id.clone(),
            goal: mission.goal.clone(),
            phase: mission.phase.as_str().into(),
            memory_pack: memory.context_pack()?,
            repair_hint: mission.last_error.clone(),
        };
        let turn = adapter.run_turn(&self.store.root, &input)?;
        memory.append(
            "backend_turn",
            &turn.summary,
            serde_json::json!({
                "backend": turn.backend,
                "claimed_tests_passed": turn.claimed_tests_passed,
                "proposed_claims": turn.proposed_claims,
            }),
        )?;

        // CRITICAL: ignore claimed_tests_passed from backend
        if turn.claimed_tests_passed {
            println!(
                "{} backend self-reported tests passed — {}",
                "!".red().bold(),
                "ignored (not evidence)".red()
            );
            memory.append(
                "untrusted_claim",
                "backend claimed tests_passed without Prove receipt",
                serde_json::json!({}),
            )?;
        }

        // Normalize paths so Windows adapters and Unix receipts agree.
        let changed: Vec<String> = turn
            .changed_files
            .iter()
            .map(|f| git_state::normalize_path_str(f))
            .filter(|f| !f.is_empty())
            .collect();

        locks.acquire(&mission.id, &changed)?;
        mission.touched_files = changed.clone();
        mission.steps_used += 1;

        let producer = Producer {
            backend: turn.backend.clone(),
            run_id: format!("run_{}", Uuid::new_v4().simple()),
        };
        let patch_receipt = receipts::mint_patch_receipt(
            &self.store.root,
            &mission.id,
            &self.policy,
            producer,
            &changed,
        )?;
        receipts.save(&patch_receipt)?;
        let tr = lifecycle::apply(mission.phase, TransitionEvent::PatchOk);
        if !tr.admitted {
            bail!("lifecycle refused patch advance: {}", tr.reason);
        }
        mission.phase = tr.to;
        mission.updated_at = chrono::Utc::now();
        println!("{} {}", "→".cyan(), tr.reason);
        println!("  {}", turn.summary.dimmed());
        Ok(())
    }

    fn do_test(
        &self,
        mission: &mut Mission,
        memory: &MissionMemory,
        receipts: &ReceiptStore,
    ) -> Result<()> {
        println!("{} running proof gate: test", "●".yellow());
        let producer = Producer {
            backend: "prove-verify".into(),
            run_id: format!("verify_{}", Uuid::new_v4().simple()),
        };
        let (receipt, admit) = receipts::mint_test_receipt(
            &self.store.root,
            &mission.id,
            &self.policy,
            producer,
            &mission.touched_files,
        )?;
        receipts.save(&receipt)?;
        mission.steps_used += 1;

        match admit {
            Ok(()) => {
                memory.append(
                    "test_receipt_admitted",
                    &receipt.receipt_id,
                    serde_json::json!({"receipt_id": receipt.receipt_id}),
                )?;
                let tr = lifecycle::apply(mission.phase, TransitionEvent::TestReceiptAdmitted);
                mission.phase = tr.to;
                mission.last_error = None;
                println!(
                    "{} test receipt admitted ({})",
                    "✓".green().bold(),
                    receipt.receipt_id
                );
            }
            Err(e) => {
                mission.repair_count += 1;
                mission.last_error = Some(e.message.clone());
                memory.append(
                    "test_receipt_rejected",
                    &e.message,
                    serde_json::json!({"kind": format!("{:?}", e.kind)}),
                )?;
                println!(
                    "{} test receipt REJECTED: {}",
                    "✗".red().bold(),
                    receipts::status_excerpt(&e.message)
                );
                println!(
                    "{} refusing done — {}",
                    "PROOF-OR-STOP".red().bold(),
                    "evidence missing or failed".yellow()
                );
                let tr = lifecycle::apply(mission.phase, TransitionEvent::Repair);
                mission.phase = tr.to;
            }
        }
        mission.updated_at = chrono::Utc::now();
        Ok(())
    }

    fn do_review(
        &self,
        mission: &mut Mission,
        memory: &MissionMemory,
        receipts: &ReceiptStore,
    ) -> Result<()> {
        println!("{} running proof gate: review checklist", "●".yellow());
        let mut notes = Vec::new();
        let mut ok = true;

        for req in &self.policy.gates.review.require {
            match req.as_str() {
                "diff_non_empty" => {
                    if mission.touched_files.is_empty() {
                        ok = false;
                        notes.push("diff_non_empty: no touched files".into());
                    } else {
                        notes.push(format!(
                            "diff_non_empty: ok ({})",
                            mission.touched_files.join(", ")
                        ));
                    }
                }
                "tests_fresh" => {
                    match receipts.latest(&mission.id, ClaimType::TestsPassed)? {
                        Some(r) => match receipts::admit_test_receipt(
                            &self.store.root,
                            &self.policy,
                            &r,
                        )? {
                            Ok(()) => notes.push(format!("tests_fresh: ok ({})", r.receipt_id)),
                            Err(e) => {
                                ok = false;
                                notes.push(format!(
                                    "tests_fresh: {}",
                                    receipts::status_excerpt(&e.message)
                                ));
                            }
                        },
                        None => {
                            ok = false;
                            notes.push("tests_fresh: missing test receipt".into());
                        }
                    }
                }
                "no_todo_marker" => {
                    let mut found = false;
                    for f in &mission.touched_files {
                        let path = self.store.root.join(f);
                        if path.exists() {
                            let text = std::fs::read_to_string(&path).unwrap_or_default();
                            if text.contains("TODO(prove-block)") {
                                found = true;
                            }
                        }
                    }
                    if found {
                        ok = false;
                        notes.push("no_todo_marker: found TODO(prove-block)".into());
                    } else {
                        notes.push("no_todo_marker: ok".into());
                    }
                }
                other => notes.push(format!("{other}: skipped (unknown check)")),
            }
        }

        let producer = Producer {
            backend: "prove-review".into(),
            run_id: format!("review_{}", Uuid::new_v4().simple()),
        };
        let note = notes.join("; ");
        let (receipt, admit) = receipts::mint_review_receipt(
            &self.store.root,
            &mission.id,
            &self.policy,
            producer,
            &mission.touched_files,
            ok,
            &note,
        )?;
        receipts.save(&receipt)?;
        mission.steps_used += 1;

        match admit {
            Ok(()) => {
                memory.append("review_admitted", &note, serde_json::json!({}))?;
                let tr = lifecycle::apply(mission.phase, TransitionEvent::ReviewReceiptAdmitted);
                mission.phase = tr.to;
                println!("{} review admitted — mission DONE", "✓".green().bold());
            }
            Err(e) => {
                mission.last_error = Some(e.message.clone());
                memory.append("review_rejected", &e.message, serde_json::json!({}))?;
                println!(
                    "{} review rejected: {}",
                    "✗".red().bold(),
                    receipts::status_excerpt(&e.message)
                );
                let tr = lifecycle::apply(mission.phase, TransitionEvent::ChangesRequested);
                mission.phase = tr.to;
            }
        }
        mission.updated_at = chrono::Utc::now();
        Ok(())
    }

    fn stop(&self, mission: &mut Mission, reason: &str) -> Result<()> {
        let tr = lifecycle::apply(
            mission.phase,
            TransitionEvent::Stop {
                reason: reason.into(),
            },
        );
        mission.phase = tr.to;
        mission.stop_reason = Some(reason.into());
        mission.updated_at = chrono::Utc::now();
        self.store
            .append_event("stop", reason)?;
        self.store.save_mission(mission)?;
        println!("{} {}", "STOP".red().bold(), reason);
        Ok(())
    }

    /// Run policy test gates against the current tree.
    /// Returns Ok(true) if admitted, Ok(false) if rejected (without Err).
    pub fn verify_only(&self) -> Result<bool> {
        self.verify_with_options(false, false)
    }

    /// CI-oriented verify.
    /// - `require_done`: also require active mission phase is Done/PrReady and fresh review receipt
    /// - `json`: emit machine-readable summary line as JSON on stdout (human lines still on stderr-ish via println)
    pub fn verify_with_options(&self, require_done: bool, json: bool) -> Result<bool> {
        let mission = self.store.load_mission().ok();
        let mission_id = mission
            .as_ref()
            .map(|m| m.id.clone())
            .unwrap_or_else(|| format!("adhoc_{}", uuid::Uuid::new_v4().simple()));

        let receipts = ReceiptStore::open(&self.store.prove_dir)?;
        let (receipt, admit) = receipts::verify_now(
            &self.store.root,
            &mission_id,
            &self.policy,
            "prove-verify",
        )?;
        receipts.save(&receipt)?;

        let mut admitted = admit.is_ok();
        let mut reasons: Vec<String> = Vec::new();

        match &admit {
            Ok(()) => {
                println!(
                    "{} verify admitted — receipt {}",
                    "✓".green().bold(),
                    receipt.receipt_id
                );
            }
            Err(e) => {
                reasons.push(e.message.clone());
                println!(
                    "{} verify REJECTED: {}",
                    "✗".red().bold(),
                    receipts::status_excerpt(&e.message)
                );
                println!(
                    "{}",
                    "Refusing done. Agents can claim. Only evidence can advance.".yellow()
                );
            }
        }

        if require_done {
            match &mission {
                None => {
                    admitted = false;
                    reasons.push("require-done: no active mission".into());
                    println!(
                        "{} require-done failed: no active mission",
                        "✗".red().bold()
                    );
                }
                Some(m) => {
                    if !matches!(m.phase, Phase::Done | Phase::PrReady) {
                        admitted = false;
                        reasons.push(format!("require-done: phase is {}", m.phase.as_str()));
                        println!(
                            "{} require-done failed: phase is {}",
                            "✗".red().bold(),
                            phase_color(m.phase)
                        );
                    } else {
                        match receipts.latest(&m.id, ClaimType::ReviewOk)? {
                            Some(_) => {
                                println!(
                                    "{} require-done: mission {} is {}",
                                    "✓".green().bold(),
                                    m.id,
                                    m.phase.as_str()
                                );
                            }
                            None => {
                                admitted = false;
                                reasons.push("require-done: missing review receipt".into());
                                println!(
                                    "{} require-done failed: missing review receipt",
                                    "✗".red().bold()
                                );
                            }
                        }
                    }
                }
            }
        }

        if json {
            let payload = serde_json::json!({
                "admitted": admitted,
                "receipt_id": receipt.receipt_id,
                "mission_id": mission_id,
                "head_hash": receipt.head_hash,
                "tree_hash": receipt.tree_hash,
                "policy_hash": receipt.policy_hash,
                "command_set_hash": receipt.command_set_hash,
                "require_done": require_done,
                "reasons": reasons,
            });
            println!("PROVE_JSON:{}", payload);
        }

        Ok(admitted)
    }

    pub fn print_status(&self, mission: &Mission) -> Result<()> {
        println!();
        println!("{}", "══ prove status ══".bold());
        println!("mission : {}", mission.id);
        println!("goal    : {}", mission.goal);
        println!("phase   : {}", phase_color(mission.phase));
        println!("backend : {}", mission.backend);
        println!(
            "budget  : steps {}/{}  repairs {}/{}",
            mission.steps_used,
            self.policy.budgets.max_steps,
            mission.repair_count,
            self.policy.gates.test.repair_limit
        );
        if !mission.touched_files.is_empty() {
            println!("touched : {}", mission.touched_files.join(", "));
        }
        if let Some(e) = &mission.last_error {
            // Status display only — keep huge pytest noise out of the matrix.
            println!("error   : {}", receipts::status_excerpt(e).red());
        }
        if let Some(s) = &mission.stop_reason {
            println!("stop    : {}", s.yellow());
        }

        let receipts = ReceiptStore::open(&self.store.prove_dir)?;
        let all = receipts.list_for_mission(&mission.id)?;
        let patch = receipts.latest(&mission.id, ClaimType::PatchApplied)?;
        let test = receipts.latest(&mission.id, ClaimType::TestsPassed)?;
        let review = receipts.latest(&mission.id, ClaimType::ReviewOk)?;

        println!("{}", "evidence:".bold());
        println!("  plan   {}", gate_mark(true, "admitted"));
        println!(
            "  patch  {}",
            match &patch {
                Some(r) => gate_mark(true, &r.receipt_id),
                None => gate_mark(false, "missing"),
            }
        );
        match &test {
            Some(r) => match receipts::admit_test_receipt(&self.store.root, &self.policy, r)? {
                Ok(()) => println!(
                    "  test   {}",
                    gate_mark(true, &format!("{} fresh", r.receipt_id))
                ),
                Err(e) => println!(
                    "  test   {}",
                    gate_mark(
                        false,
                        &format!(
                            "{} stale/fail: {}",
                            r.receipt_id,
                            receipts::status_excerpt(&e.message)
                        )
                    )
                ),
            },
            None => println!("  test   {}", gate_mark(false, "missing")),
        }
        println!(
            "  review {}",
            match &review {
                Some(r) => gate_mark(true, &r.receipt_id),
                None => gate_mark(false, "missing"),
            }
        );
        println!(
            "  done   {}",
            gate_mark(
                matches!(mission.phase, Phase::Done | Phase::PrReady),
                mission.phase.as_str()
            )
        );

        println!("receipts: {}", all.len());
        for r in all {
            println!(
                "  - {:?} {} (tree {})",
                r.claim_type,
                r.receipt_id,
                &r.tree_hash.chars().take(8).collect::<String>()
            );
        }

        println!("{}", "next:".bold());
        match mission.phase {
            Phase::Done => {
                println!("  {}", "prove pr       # export admissible evidence bundle".dimmed());
                println!("  {}", "prove verify   # re-check gates against HEAD".dimmed());
            }
            Phase::PrReady => {
                println!("  {}", "prove pr       # re-export bundle".dimmed());
                println!("  {}", "open the PR_EVIDENCE.md under .prove/artifacts/".dimmed());
            }
            Phase::Failed => {
                println!("  {}", "prove run \"<goal>\" --backend local-loop".dimmed());
                println!("  {}", "prove doctor   # check env / policy / backends".dimmed());
            }
            Phase::Testing | Phase::Reviewing | Phase::Patching | Phase::Planned => {
                println!("  {}", "prove resume   # continue the active mission".dimmed());
                println!("  {}", "prove verify   # re-run test gates only".dimmed());
                println!("  {}", "prove status   # refresh this matrix".dimmed());
            }
        }
        println!(
            "{}",
            "Agents can claim. Only evidence can advance.".dimmed()
        );
        Ok(())
    }

    pub fn export_pr_bundle(&self) -> Result<std::path::PathBuf> {
        let mission = self.store.load_mission()?;
        if !matches!(mission.phase, Phase::Done | Phase::PrReady) {
            bail!(
                "refusing PR bundle: mission phase is '{}' (need 'done' or 'pr_ready')\n  \
                 Proof-or-stop: agents cannot export without admitted evidence.\n  \
                 Fix: `prove resume` or `prove run \"...\"` until phase is done, then `prove pr`",
                mission.phase.as_str()
            );
        }
        let receipts = ReceiptStore::open(&self.store.prove_dir)?;
        let all = receipts.list_for_mission(&mission.id)?;
        let test = receipts
            .latest(&mission.id, ClaimType::TestsPassed)?
            .ok_or_else(|| {
                anyhow!(
                    "missing test receipt for mission {}\n  Fix: `prove verify` then retry `prove pr`",
                    mission.id
                )
            })?;
        match receipts::admit_test_receipt(&self.store.root, &self.policy, &test)? {
            Ok(()) => {}
            Err(e) => bail!(
                "test receipt not admissible: {}\n  Fix: `prove verify` after fixing failures",
                receipts::status_excerpt(&e.message)
            ),
        }
        let review = receipts
            .latest(&mission.id, ClaimType::ReviewOk)?
            .ok_or_else(|| {
                anyhow!(
                    "missing review receipt for mission {}\n  Fix: finish review gate via `prove resume`",
                    mission.id
                )
            })?;

        let dir = self.store.prove_dir.join("artifacts");
        std::fs::create_dir_all(&dir)?;
        let json_path = dir.join(format!("{}_pr_bundle.json", mission.id));
        let md_path = dir.join(format!("{}_PR_EVIDENCE.md", mission.id));

        let bundle = serde_json::json!({
            "mission": mission,
            "receipts": all,
            "exported_at": chrono::Utc::now(),
            "trust_model": "Do not trust agent prose without these receipts.",
            "admissibility": {
                "test_receipt": test.receipt_id,
                "review_receipt": review.receipt_id,
                "policy_hash": self.policy.policy_hash(),
            }
        });
        std::fs::write(&json_path, serde_json::to_string_pretty(&bundle)?)?;

        let mut md = String::new();
        md.push_str("# Prove PR Evidence Bundle\n\n");
        md.push_str(&format!("**Mission:** `{}`  \n", mission.id));
        md.push_str(&format!("**Goal:** {}  \n", mission.goal));
        md.push_str(&format!("**Backend:** {}  \n", mission.backend));
        md.push_str(&format!("**Phase:** {}  \n\n", mission.phase.as_str()));
        md.push_str("## Trust notice\n\n");
        md.push_str("> Agents can claim. Only evidence can advance.\n\n");
        md.push_str("## Touched files\n\n");
        if mission.touched_files.is_empty() {
            md.push_str("- _(none recorded)_\n");
        } else {
            for f in &mission.touched_files {
                md.push_str(&format!("- `{f}`\n"));
            }
        }
        md.push_str("\n## Receipts\n\n");
        md.push_str("| Claim | Receipt ID | Tree |\n|---|---|---|\n");
        for r in &all {
            md.push_str(&format!(
                "| {:?} | `{}` | `{}` |\n",
                r.claim_type,
                r.receipt_id,
                &r.tree_hash.chars().take(12).collect::<String>()
            ));
        }
        md.push_str("\n## Test commands (policy)\n\n");
        for cmd in &self.policy.gates.test.commands {
            md.push_str(&format!("- `{}`\n", cmd.join(" ")));
        }
        md.push_str(&format!(
            "\n## Hashes\n\n- policy: `{}`\n- command_set: `{}`\n- test receipt: `{}`\n- review receipt: `{}`\n",
            self.policy.policy_hash(),
            self.policy.command_set_hash(),
            test.receipt_id,
            review.receipt_id
        ));
        md.push_str("\n---\n_Generated by Prove. Re-run `prove verify` before merge if the tree moves._\n");
        std::fs::write(&md_path, md)?;

        let mut m = mission;
        let tr = lifecycle::apply(m.phase, TransitionEvent::BundleOk);
        if tr.admitted {
            m.phase = tr.to;
            self.store.save_mission(&m)?;
        }
        println!("{} wrote {}", "✓".green().bold(), json_path.display());
        println!("{} wrote {}", "✓".green().bold(), md_path.display());
        println!(
            "{} phase → {}",
            "→".cyan(),
            phase_color(m.phase)
        );
        Ok(json_path)
    }
}

fn gate_mark(ok: bool, detail: &str) -> String {
    if ok {
        format!("{} {}", "✓".green().bold(), detail)
    } else {
        format!("{} {}", "·".yellow(), detail.dimmed())
    }
}

fn phase_color(p: Phase) -> colored::ColoredString {
    match p {
        Phase::Done | Phase::PrReady => p.as_str().green().bold(),
        Phase::Failed => p.as_str().red().bold(),
        Phase::Testing | Phase::Reviewing => p.as_str().yellow().bold(),
        _ => p.as_str().cyan().bold(),
    }
}

