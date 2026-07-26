use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Planned,
    Patching,
    Testing,
    Reviewing,
    Done,
    PrReady,
    Failed,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Planned => "planned",
            Phase::Patching => "patching",
            Phase::Testing => "testing",
            Phase::Reviewing => "reviewing",
            Phase::Done => "done",
            Phase::PrReady => "pr_ready",
            Phase::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionEvent {
    PlanOk,
    PatchOk,
    TestReceiptAdmitted,
    ReviewReceiptAdmitted,
    BundleOk,
    Repair,
    ChangesRequested,
    Stop { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionResult {
    pub from: Phase,
    pub to: Phase,
    pub admitted: bool,
    pub reason: String,
}

/// Pure lifecycle: claims never auto-advance without explicit admitted evidence events.
pub fn apply(phase: Phase, event: TransitionEvent) -> TransitionResult {
    match (phase, event) {
        (Phase::Planned, TransitionEvent::PlanOk) => ok(phase, Phase::Patching, "plan admitted"),
        (Phase::Patching, TransitionEvent::PatchOk) => ok(phase, Phase::Testing, "patch receipt admitted"),
        (Phase::Testing, TransitionEvent::TestReceiptAdmitted) => {
            ok(phase, Phase::Reviewing, "test receipt admitted")
        }
        (Phase::Reviewing, TransitionEvent::ReviewReceiptAdmitted) => {
            ok(phase, Phase::Done, "review receipt admitted")
        }
        (Phase::Done, TransitionEvent::BundleOk) => ok(phase, Phase::PrReady, "bundle admitted"),
        (Phase::Testing, TransitionEvent::Repair) => {
            ok(phase, Phase::Patching, "verify failed — repair")
        }
        (Phase::Reviewing, TransitionEvent::ChangesRequested) => {
            ok(phase, Phase::Patching, "review requested changes")
        }
        (Phase::Patching, TransitionEvent::Stop { reason })
        | (Phase::Testing, TransitionEvent::Stop { reason })
        | (Phase::Reviewing, TransitionEvent::Stop { reason })
        | (Phase::Planned, TransitionEvent::Stop { reason }) => TransitionResult {
            from: phase,
            to: Phase::Failed,
            admitted: true,
            reason,
        },
        (from, ev) => TransitionResult {
            from,
            to: from,
            admitted: false,
            reason: format!("illegal transition from {:?} via {:?}", from, ev),
        },
    }
}

fn ok(from: Phase, to: Phase, reason: &str) -> TransitionResult {
    TransitionResult {
        from,
        to,
        admitted: true,
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cannot_skip_to_done() {
        let r = apply(Phase::Patching, TransitionEvent::ReviewReceiptAdmitted);
        assert!(!r.admitted);
        assert_eq!(r.to, Phase::Patching);
    }

    #[test]
    fn happy_path() {
        let mut p = Phase::Planned;
        for ev in [
            TransitionEvent::PlanOk,
            TransitionEvent::PatchOk,
            TransitionEvent::TestReceiptAdmitted,
            TransitionEvent::ReviewReceiptAdmitted,
            TransitionEvent::BundleOk,
        ] {
            let r = apply(p, ev);
            assert!(r.admitted, "{}", r.reason);
            p = r.to;
        }
        assert_eq!(p, Phase::PrReady);
    }
}
