use clap::{Parser, Subcommand};
use colored::Colorize;
use prove::adapters;
use prove::orchestrator::Orchestrator;
use prove::policy::Policy;
use prove::store::ProveStore;
use std::env;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "prove",
    version,
    about = "Evidence-gated multi-CLI control plane for coding agents",
    long_about = "Evidence-gated multi-CLI control plane for coding agents.\n\
Agents can claim. Only evidence can advance.\n\n\
Quick start:\n  prove init\n  prove doctor\n  prove run \"fix the bug\" --backend local-loop\n  prove status\n  prove pr\n  prove eval traps"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize .prove/ policy, adapters, and stores in the current repo
    Init,
    /// Run a mission under proof-or-stop gates
    Run {
        /// Mission goal
        goal: String,
        /// Pin backend: local-loop | naive | claude-code | aider | codex
        #[arg(long)]
        backend: Option<String>,
    },
    /// Show current mission status and evidence matrix
    Status,
    /// Re-run verify gates against HEAD (does not trust agent logs)
    Verify {
        /// Exit non-zero when verification is rejected (for CI)
        #[arg(long)]
        ci: bool,
        /// Emit PROVE_JSON:{...} machine-readable summary
        #[arg(long)]
        json: bool,
        /// Also require active mission phase Done/PrReady + review receipt
        #[arg(long)]
        require_done: bool,
    },
    /// Resume the active mission
    Resume,
    /// Export admissible PR evidence bundle (JSON + markdown); refuses if not done
    Pr,
    /// Healthcheck environment, policy, git, and backends; suggests next commands
    Doctor,
    /// Run the public trap suite (false-done metrics)
    Eval {
        #[arg(default_value = "traps")]
        suite: String,
    },
    /// Healthcheck configured backends
    Adapters {
        #[command(subcommand)]
        cmd: AdapterCmd,
    },
    /// Show active policy
    Policy {
        #[command(subcommand)]
        cmd: PolicyCmd,
    },
    /// Manage local receipt sealing keys
    Keys {
        #[command(subcommand)]
        cmd: KeysCmd,
    },
}

#[derive(Subcommand, Debug)]
enum KeysCmd {
    /// Generate a local seal key (.prove/keys). Default alg: ed25519
    Init {
        /// Seal algorithm: ed25519 (default) or hmac-sha256
        #[arg(long, default_value = "ed25519")]
        alg: String,
    },
    /// Show whether a local seal key is configured
    Status,
    /// Print the local public key (ed25519) for multi-party handoff
    Pubkey,
}

#[derive(Subcommand, Debug)]
enum AdapterCmd {
    /// Probe each backend binary / healthcheck
    Test,
}

#[derive(Subcommand, Debug)]
enum PolicyCmd {
    /// Print loaded policy.yml and content hashes
    Show,
}

fn main() {
    if let Err(e) = real_main() {
        print_error(&e);
        std::process::exit(1);
    }
}

/// Prefer a single clear message over deep anyhow chains.
fn print_error(err: &anyhow::Error) {
    eprintln!("{} {}", "error:".red().bold(), err);
    // Show at most one extra cause line when it adds real detail.
    let mut causes = err.chain().skip(1);
    if let Some(cause) = causes.next() {
        let top = err.to_string();
        let cause_s = cause.to_string();
        if !top.contains(&cause_s) {
            eprintln!("  {}", cause_s.dimmed());
        }
    }
}

fn real_main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cwd = env::current_dir()?;

    match cli.command {
        Commands::Init => {
            let store = ProveStore::discover(&cwd)?;
            store.init(&Policy::default())?;
            println!(
                "{} initialized {}",
                "✓".green().bold(),
                store.prove_dir.display()
            );
            println!("{}", "Don't trust the agent. Trust the evidence.".cyan());
            println!("{}", "next:".bold());
            println!("  {}", "prove doctor".dimmed());
            println!(
                "  {}",
                "prove run \"<goal>\" --backend local-loop".dimmed()
            );
        }
        Commands::Run { goal, backend } => {
            let store = ProveStore::discover(&cwd)?;
            store.ensure_initialized()?;
            let orch = Orchestrator::new(&store)?;
            let chosen = backend
                .clone()
                .unwrap_or_else(|| prove::adapters::route_backend(&goal, None));
            println!("{}", "prove run".bold());
            println!("goal   : {goal}");
            println!("backend: {chosen}");
            let mission = orch.run_mission(&goal, backend.as_deref())?;
            if matches!(
                mission.phase,
                prove::lifecycle::Phase::Done | prove::lifecycle::Phase::PrReady
            ) {
                println!(
                    "{}",
                    "Mission complete with admissible evidence.".green().bold()
                );
                println!("{}", "Next: prove pr   # export evidence bundle".dimmed());
            } else if matches!(mission.phase, prove::lifecycle::Phase::Failed) {
                println!(
                    "{}",
                    "Mission stopped without fake-done.".yellow().bold()
                );
                println!("{}", "Next: prove doctor   # inspect env / policy".dimmed());
            }
        }
        Commands::Status => {
            let store = ProveStore::discover(&cwd)?;
            let orch = Orchestrator::new(&store)?;
            let mission = store.load_mission()?;
            orch.print_status(&mission)?;
        }
        Commands::Verify {
            ci,
            json,
            require_done,
        } => {
            let store = ProveStore::discover(&cwd)?;
            // CI can run with existing policy or auto-init defaults for greenfield.
            if ci {
                store.ensure_initialized()?;
            } else {
                store.require_initialized()?;
            }
            let orch = Orchestrator::new(&store)?;
            let admitted = orch.verify_with_options(require_done, json)?;
            if ci && !admitted {
                eprintln!(
                    "{}",
                    "prove verify --ci: gates not admitted (exit 1)".red().bold()
                );
                std::process::exit(1);
            }
        }
        Commands::Resume => {
            let store = ProveStore::discover(&cwd)?;
            let orch = Orchestrator::new(&store)?;
            orch.resume()?;
        }
        Commands::Pr => {
            let store = ProveStore::discover(&cwd)?;
            let orch = Orchestrator::new(&store)?;
            orch.export_pr_bundle()?;
        }
        Commands::Doctor => {
            run_doctor(&cwd)?;
        }
        Commands::Eval { suite } => {
            if suite != "traps" {
                anyhow::bail!(
                    "unknown suite '{suite}'\n  \
                     Available: traps\n  \
                     Usage: prove eval traps"
                );
            }
            let repo = find_prove_source_root(&cwd)?;
            let results = prove::eval::run_trap_suite(&repo)?;
            prove::eval::print_eval_report(&results);
            if results.iter().any(|r| r.prove_false_done) {
                std::process::exit(2);
            }
        }
        Commands::Adapters { cmd } => match cmd {
            AdapterCmd::Test => {
                println!("{}", "══ adapters ══".bold());
                for (id, health) in adapters::list_adapter_health() {
                    match health {
                        Ok(msg) => println!("{} {id}: {msg}", "✓".green()),
                        Err(err) => println!("{} {id}: {err}", "·".yellow()),
                    }
                }
                println!("{}", "next:".bold());
                println!(
                    "  {}",
                    "prove run \"...\" --backend local-loop|claude-code|aider|codex".dimmed()
                );
            }
        },
        Commands::Policy { cmd } => match cmd {
            PolicyCmd::Show => {
                let store = ProveStore::discover(&cwd)?;
                store.require_initialized()?;
                let p = store.load_policy()?;
                println!("{}", serde_yaml::to_string(&p)?);
                println!("policy_hash: {}", p.policy_hash());
                println!("command_set_hash: {}", p.command_set_hash());
            }
        },
        Commands::Keys { cmd } => match cmd {
            KeysCmd::Init { alg } => {
                let store = ProveStore::discover(&cwd)?;
                store.ensure_initialized()?;
                let alg = prove::seal::SealAlg::parse(&alg)?;
                let key = prove::seal::LocalKey::init(&store.prove_dir, alg)?;
                println!(
                    "{} seal key ready alg={} key_id={} at {}",
                    "✓".green().bold(),
                    key.alg().as_str(),
                    key.key_id(),
                    key.path().display()
                );
                if let Some(pk) = key.public_key_hex() {
                    println!("public  {}", pk);
                }
                println!(
                    "{}",
                    "New receipts will be sealed. Keep .prove/keys/* private.".dimmed()
                );
            }
            KeysCmd::Status => {
                let store = ProveStore::discover(&cwd)?;
                match prove::seal::LocalKey::load(&store.prove_dir)? {
                    Some(k) => {
                        println!(
                            "{} seal key present alg={} key_id={} path={}",
                            "✓".green().bold(),
                            k.alg().as_str(),
                            k.key_id(),
                            k.path().display()
                        );
                        if let Some(pk) = k.public_key_hex() {
                            println!("public  {}", &pk[..16.min(pk.len())]);
                        }
                    }
                    None => println!(
                        "{} no seal key — run `prove keys init`",
                        "·".yellow()
                    ),
                }
            }
            KeysCmd::Pubkey => {
                let store = ProveStore::discover(&cwd)?;
                match prove::seal::LocalKey::load(&store.prove_dir)? {
                    Some(k) => match k.public_key_hex() {
                        Some(pk) => {
                            println!("alg     {}", k.alg().as_str());
                            println!("key_id  {}", k.key_id());
                            println!("public  {pk}");
                        }
                        None => anyhow::bail!(
                            "current key alg {} has no public key (use ed25519)",
                            k.alg().as_str()
                        ),
                    },
                    None => anyhow::bail!("no seal key — run `prove keys init --alg ed25519`"),
                }
            }
        },
    }
    Ok(())
}

fn run_doctor(cwd: &std::path::Path) -> anyhow::Result<()> {
    println!("{}", "══ prove doctor ══".bold());
    println!("cwd: {}", cwd.display());

    let git_ok = cwd.join(".git").exists()
        || std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(cwd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    println!(
        "git     {}",
        if git_ok {
            "✓ ok".green().to_string()
        } else {
            "· not a git repo (Prove can init local git fingerprints)".yellow().to_string()
        }
    );

    let store = ProveStore::discover(cwd)?;
    let mut has_policy = false;
    let mut policy_ok = false;
    if store.prove_dir.join("policy.yml").exists() {
        has_policy = true;
        match store.load_policy() {
            Ok(p) => {
                policy_ok = true;
                println!(
                    "policy  {} ({})",
                    "✓ loaded".green(),
                    store.prove_dir.join("policy.yml").display()
                );
                println!("  commands: {}", p.gates.test.commands.len());
                let ph = p.policy_hash();
                println!("  policy_hash: {}", &ph[..12.min(ph.len())]);
            }
            Err(e) => {
                println!("policy  {} {}", "✗ invalid".red().bold(), e);
            }
        }
    } else {
        println!("policy  {}", "· missing — run prove init".yellow());
    }

    println!("{}", "backends:".bold());
    for (id, health) in adapters::list_adapter_health() {
        match health {
            Ok(msg) => println!("  {} {id}: {msg}", "✓".green()),
            Err(err) => println!("  {} {id}: {err}", "·".yellow()),
        }
    }

    let mut phase: Option<String> = None;
    if store.mission_path().exists() {
        match store.load_mission() {
            Ok(m) => {
                phase = Some(m.phase.as_str().to_string());
                println!(
                    "mission {} phase={}",
                    "✓ active".green(),
                    m.phase.as_str()
                );
            }
            Err(e) => {
                println!("mission {} {}", "✗ unreadable".red(), e);
            }
        }
    } else {
        println!("mission {}", "· none".dimmed());
    }

    println!();
    println!("{}", "next:".bold());
    if !has_policy || !policy_ok {
        println!("  {}", "prove init".cyan());
        println!(
            "  {}",
            "prove run \"fix the failing test\" --backend local-loop".dimmed()
        );
    } else {
        match phase.as_deref() {
            Some("done") => {
                println!("  {}", "prove pr".cyan());
                println!("  {}", "prove verify".dimmed());
            }
            Some("pr_ready") => {
                println!("  {}", "prove pr          # re-export bundle".cyan());
                println!(
                    "  {}",
                    "open .prove/artifacts/*_PR_EVIDENCE.md".dimmed()
                );
            }
            Some("failed") => {
                println!(
                    "  {}",
                    "prove run \"<goal>\" --backend local-loop".cyan()
                );
                println!("  {}", "prove status".dimmed());
            }
            Some(_) => {
                println!("  {}", "prove status".cyan());
                println!("  {}", "prove resume".dimmed());
                println!("  {}", "prove verify".dimmed());
            }
            None => {
                println!(
                    "  {}",
                    "prove run \"<goal>\" --backend local-loop".cyan()
                );
                println!("  {}", "prove adapters test".dimmed());
                println!(
                    "  {}",
                    "prove eval traps   # from the prove source repo".dimmed()
                );
            }
        }
    }

    if has_policy && policy_ok {
        if let Ok(p) = store.load_policy() {
            println!(
                "sandbox {}",
                prove::sandbox::describe(&p.sandbox_opts()).dimmed()
            );
        }
    }

    println!("{}", "Don't trust the agent. Trust the evidence.".cyan());
    Ok(())
}

fn find_prove_source_root(start: &std::path::Path) -> anyhow::Result<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join("fixtures").join("traps").exists() {
            return Ok(cur);
        }
        if !cur.pop() {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            if manifest.join("fixtures").join("traps").exists() {
                return Ok(manifest);
            }
            anyhow::bail!(
                "could not locate fixtures/traps\n  \
                 `prove eval traps` must be run from the Prove source repository\n  \
                 (expected a fixtures/traps directory up the tree)"
            );
        }
    }
}


