//! The anchor separability gate — the go/no-go experiment for pure
//! geometry (Task 22).
//!
//! Two variants, one question:
//!
//! - **Hermetic** (always runs): synthetic exemplars with known geometry
//!   prove the *metric code* is correct — a world built to separate
//!   must pass, a world built to collapse must fail.
//! - **Live** (`WUNICORM_LLM_LIVE=1`, the `llm_live` pattern): the real
//!   corpus embedded by the real server on this machine. This is the
//!   gate: if the real embedding space cannot hold the four poles
//!   apart, pure geometry is not trustworthy and the fallback (a
//!   learned probe) goes to the paper trail instead. The numbers print
//!   under `--nocapture` and land in D-002's Context.

use memory_proto::axes::{
    self, AxisFrame, Function, GATE_DISCRIMINABILITY_MIN, GATE_OPPOSITE_COSINE_MAX,
};
use memory_proto::llm::{LlmClient, LlmConfig, UreqClient};

fn live() -> Option<UreqClient> {
    if std::env::var("WUNICORM_LLM_LIVE").is_err() {
        return None;
    }
    Some(UreqClient::new(LlmConfig::from_env()))
}

/// Ten deterministic exemplars per pole in 4-dim space: each pole's
/// cluster hugs a basis vector with a small per-exemplar offset.
fn synthetic_exemplars(spread: f32) -> Vec<(Function, Vec<f32>)> {
    let basis = |f: Function| {
        usize::from(matches!(f, Function::Intuiting)) * 3
            + usize::from(matches!(f, Function::Sensing)) * 2
            + usize::from(matches!(f, Function::Feeling))
    };
    let mut out = vec![];
    for f in Function::ALL {
        for k in 0..10u8 {
            let mut v = vec![spread * (f32::from(k) - 4.5) / 20.0; 4];
            v[basis(f)] = 1.0;
            out.push((f, v));
        }
    }
    out
}

#[test]
fn hermetic_separable_world_passes_the_gate() {
    let exemplars = synthetic_exemplars(0.5);
    let report = axes::separability(&exemplars).expect("frame builds from synthetic evidence");
    println!("{report}");
    assert!(report.passes(), "a world built to separate must pass");
    // The synthetic poles are near-orthogonal: opposite cosines near 0.
    assert!(
        report.opposite_cosine[0] < 0.2,
        "T·F too close: {}",
        report.opposite_cosine[0]
    );
    assert!(
        report.opposite_cosine[1] < 0.2,
        "S·N too close: {}",
        report.opposite_cosine[1]
    );
    // Every exemplar lands in its own pole at leave-one-out.
    for d in report.discriminability {
        assert_eq!(d, 1.0, "orthogonal poles hold every exemplar");
    }
}

#[test]
fn hermetic_collapsed_world_fails_the_gate() {
    // All four poles share one direction with the same asymmetric
    // spread: the spread survives centering (a frame builds — the poles
    // are nonzero), but every pole centroid is the same vector, so
    // judgment and perception exist as columns, not as geometry.
    let exemplars: Vec<(Function, Vec<f32>)> = (0..10u8)
        .flat_map(|k| {
            let kf = f32::from(k);
            Function::ALL.map(move |f| (f, vec![1.0 + 0.01 * kf * kf, 1.0 + 0.02 * kf, 1.0, 1.0]))
        })
        .collect();
    let report = axes::separability(&exemplars).expect("a frame still builds");
    println!("{report}");
    assert!(!report.passes(), "a collapsed world must fail the gate");
    assert!(
        report.opposite_cosine[0] > 0.99,
        "identical poles, identical cosines: {}",
        report.opposite_cosine[0]
    );
}

#[test]
fn hermetic_thresholds_are_what_the_docs_say() {
    // The corpus file's stated criteria and the code's constants are one
    // claim — keep them pinned together.
    assert_eq!(GATE_OPPOSITE_COSINE_MAX, 0.75);
    assert_eq!(GATE_DISCRIMINABILITY_MIN, 0.8);
    // And the frame still refuses missing poles.
    assert!(AxisFrame::from_exemplars(&synthetic_exemplars(0.1)[..10]).is_none());
}

/// THE gate. Live embeddings of the real corpus, real metrics, real
/// verdict — measured against the method as shipped, anisotropy removal
/// included (the raw space failed this gate at 0.94 opposite-pole
/// cosine; corpus-mean centering is part of `AxisFrame`, not a test-side
/// preprocessing step). Run manually with:
///
/// ```text
/// WUNICORM_LLM_LIVE=1 cargo test --test anchor_eval live -- --nocapture --test-threads=1
/// ```
#[test]
fn live_corpus_separates_the_gate() {
    let Some(client) = live() else {
        return;
    };
    let corpus = axes::corpus();
    assert_eq!(corpus.len(), 40, "the corpus is four poles of ten");

    let texts: Vec<&str> = corpus.iter().map(|e| e.text.as_str()).collect();
    let embeddings = client.embed(&texts).expect("embed the corpus");
    let evidence: Vec<(Function, Vec<f32>)> = corpus
        .into_iter()
        .zip(embeddings)
        .map(|(e, v)| (e.function, v))
        .collect();

    let report =
        axes::separability(&evidence).expect("the corpus has four poles — a frame must build");
    println!("{report}");
    assert!(
        report.passes(),
        "the real embedding space failed the gate even with anisotropy \
          removal — pure geometry is not trustworthy; record the numbers \
          in D-002 and design the probe fallback before Task 25"
    );
}

/// Gate tooling: per-exemplar leave-one-out diagnostics — which texts
/// betray their pole, and by how much. This is the calibration
/// instrument for future corpus revisions (v1's first draft failed on
/// seven cross-register exemplars; this test found them). Run manually
/// with `WUNICORM_LLM_LIVE=1 -- --nocapture --test-threads=1`.
#[test]
fn live_loo_diagnostics_per_exemplar() {
    let Some(client) = live() else {
        return;
    };
    let corpus = axes::corpus();
    let texts: Vec<String> = corpus.iter().map(|e| e.text.clone()).collect();
    let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
    let embeddings = client.embed(&refs).expect("embed corpus");
    let evidence: Vec<(Function, Vec<f32>)> = corpus
        .into_iter()
        .zip(embeddings)
        .map(|(e, v)| (e.function, v))
        .collect();

    for (k, (f, v)) in evidence.iter().enumerate() {
        let without: Vec<(Function, Vec<f32>)> = evidence
            .iter()
            .enumerate()
            .filter(|&(j, _)| j != k)
            .map(|(_, (g, w))| (*g, w.clone()))
            .collect();
        let Some(loo) = AxisFrame::from_exemplars(&without) else {
            println!("MISS {f:?} no frame");
            continue;
        };
        let (j, p) = loo.coordinates(v);
        let coord = if f.axis() == axes::Axis::Judgment {
            j
        } else {
            p
        };
        let positive = matches!(f, Function::Thinking | Function::Sensing);
        let ok = if positive { coord > 0.0 } else { coord < 0.0 };
        println!(
            "{:9} coord {:+.3} {} {}",
            f.as_str(),
            coord,
            if ok { "ok  " } else { "MISS" },
            texts[k].as_str()
        );
    }
}
