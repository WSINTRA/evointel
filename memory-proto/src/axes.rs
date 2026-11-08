//! The function axes — Jung's two bipolar pairs as geometry.
//!
//! - **judgment**: thinking ↔ feeling — how a thing is evaluated
//!   (impersonal consequence ↔ personal worth)
//! - **perception**: sensing ↔ intuiting — how a thing is taken in
//!   (concrete presence ↔ pattern and possibility)
//!
//! A pole's anchor is the centroid of its exemplar embeddings after
//! **corpus-mean centering** (anisotropy removal — the raw space failed
//! the gate at 0.94 opposite-pole cosine; centered, it passes at −0.28);
//! a raw embedding's coordinate on an axis is
//! `cos(x_c, own_pole) − cos(x_c, opposite_pole)` where `x_c` is the
//! embedding centered on the same corpus mean — signed, so positive
//! judgment-coordinate means thinking-ward and negative means
//! feeling-ward. Positive perception-coordinate is sensing-ward;
//! negative is intuiting-ward.
//!
//! This module is pure math over vectors: no DB, no client. The corpus
//! lives in `prompts/anchors-v1.md`; the gate that decides whether pure
//! geometry is trustworthy lives in `tests/anchor_eval.rs`; the
//! persistence of anchors as `FunctionAnchor` rows comes with the self
//! tables. Everything degrades gracefully — no anchors, no coordinates,
//! no self pass, the dream continues.

use std::fmt;

/// One of the four Jungian functions. The strings match the schema's
/// `function_tag` values exactly — one source of truth for the spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Function {
    Thinking,
    Feeling,
    Sensing,
    Intuiting,
}

impl Function {
    /// All four, in axis order: the judgment pair, then the perception
    /// pair.
    pub const ALL: [Function; 4] = [
        Function::Thinking,
        Function::Feeling,
        Function::Sensing,
        Function::Intuiting,
    ];

    /// The schema spelling — `Memory.function_tag`, `FunctionAnchor.function`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Function::Thinking => "thinking",
            Function::Feeling => "feeling",
            Function::Sensing => "sensing",
            Function::Intuiting => "intuiting",
        }
    }

    /// Parses the schema spelling; anything else is `None` (a validation
    /// failure at the caller, never a silent default).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "thinking" => Some(Function::Thinking),
            "feeling" => Some(Function::Feeling),
            "sensing" => Some(Function::Sensing),
            "intuiting" => Some(Function::Intuiting),
            _ => None,
        }
    }

    /// Which axis the function is a pole of.
    pub const fn axis(self) -> Axis {
        match self {
            Function::Thinking | Function::Feeling => Axis::Judgment,
            Function::Sensing | Function::Intuiting => Axis::Perception,
        }
    }

    /// The function at the other end of this function's axis.
    pub const fn opposite(self) -> Function {
        match self {
            Function::Thinking => Function::Feeling,
            Function::Feeling => Function::Thinking,
            Function::Sensing => Function::Intuiting,
            Function::Intuiting => Function::Sensing,
        }
    }
}

/// One bipolar axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    Judgment,
    Perception,
}

/// One corpus exemplar: a text and the pole it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Exemplar {
    pub function: Function,
    pub text: String,
}

/// Parses the corpus file's format: `## <function>` headings, each
/// followed by `- ` bullet exemplars. Prose before the first heading and
/// lines that are not bullets are ignored (header commentary lives in
/// the file). Unknown headings are skipped — a typo'd pole name yields a
/// short count that the corpus test catches, not a silent fifth pole.
pub fn parse_corpus(md: &str) -> Vec<Exemplar> {
    let mut out: Vec<Exemplar> = vec![];
    let mut current: Option<Function> = None;
    for line in md.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("## ") {
            current = Function::parse(rest.trim().to_lowercase().as_str());
        } else if let (Some(f), Some(text)) = (current, line.strip_prefix("- ")) {
            let text = text.trim();
            if !text.is_empty() {
                out.push(Exemplar {
                    function: f,
                    text: text.to_string(),
                });
            }
        }
    }
    out
}

/// The corpus: the runtime copy at `prompts/anchors-v1.md` when present
/// (corpus iteration without recompiling), else the copy baked into the
/// binary at build time. Both are the same file in the repo — the same
/// contract as the llm layer's prompt loading.
pub fn corpus() -> Vec<Exemplar> {
    const BAKED: &str = include_str!("../prompts/anchors-v1.md");
    match std::fs::read_to_string("prompts/anchors-v1.md") {
        Ok(text) => parse_corpus(&text),
        Err(_) => parse_corpus(BAKED),
    }
}

/// The two axes as anchor centroids. `None` when a pole has no
/// exemplars — the caller's cue to degrade, not to invent a frame.
///
/// **Anisotropy removal is part of the method.** EmbeddingGemma-2's raw
/// space failed the gate (Task 22): every pole centroid 0.93–0.95 cosine
/// from every other — a dominant common direction burying the contrast.
/// The fix is corpus-mean centering (all-but-the-top): subtract the mean
/// of all exemplar embeddings, renormalize each, *then* build pole
/// centroids — and at read time, center each incoming embedding with the
/// same stored mean before measuring. After centering, the real corpus
/// passes the gate with opposite poles at −0.28 cosine (facing away, as
/// a bipolar axis should) and leave-one-out discriminability ≥ 0.80 on
/// every pole. The mean is computed, never trained — still pure
/// geometry, and it is recomputable from the stored exemplars alone.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisFrame {
    thinking: Vec<f32>,
    feeling: Vec<f32>,
    sensing: Vec<f32>,
    intuiting: Vec<f32>,
    mean: Vec<f32>,
}

impl AxisFrame {
    /// Builds the frame from exemplar embeddings: center every vector on
    /// the corpus mean (anisotropy removal), renormalize, then take
    /// per-pole centroids. `None` if any pole is empty or the vectors
    /// disagree in width.
    pub fn from_exemplars(embeddings: &[(Function, Vec<f32>)]) -> Option<Self> {
        let dim = embeddings.first().map(|(_, v)| v.len())?;
        // The corpus mean — computed, never trained.
        let mut mean = vec![0f32; dim];
        for (_, v) in embeddings {
            if v.is_empty() || v.len() != dim {
                return None;
            }
            for (m, x) in mean.iter_mut().zip(v.iter()) {
                *m += x;
            }
        }
        #[allow(clippy::cast_precision_loss)] // exemplar counts stay far below 2^24
        let n = embeddings.len() as f32;
        for m in &mut mean {
            *m /= n;
        }
        // Center + renormalize each exemplar, grouped by pole.
        let mut by_pole: [Vec<Vec<f32>>; 4] = [vec![], vec![], vec![], vec![]];
        for (f, v) in embeddings {
            by_pole[Self::pole_index(*f)].push(centered(v, &mean));
        }
        let [t, f, s, n] = by_pole;
        Some(Self {
            thinking: centroid(&t)?,
            feeling: centroid(&f)?,
            sensing: centroid(&s)?,
            intuiting: centroid(&n)?,
            mean,
        })
    }

    /// A raw embedding's coordinate pair: `(judgment, perception)`. The
    /// input is centered on the frame's corpus mean first — the same
    /// anisotropy removal the poles were built under. Positive judgment
    /// is thinking-ward; positive perception is sensing-ward. Zero
    /// vectors and width mismatches yield `(0.0, 0.0)` — no signal, not
    /// NaN, not manufactured signal.
    pub fn coordinates(&self, embedding: &[f32]) -> (f32, f32) {
        if embedding.len() != self.mean.len() {
            return (0.0, 0.0);
        }
        let x = centered(embedding, &self.mean);
        let judgment = cosine(&x, &self.thinking) - cosine(&x, &self.feeling);
        let perception = cosine(&x, &self.sensing) - cosine(&x, &self.intuiting);
        (judgment, perception)
    }

    /// The anchor for a pole, in **centered space** — for inter-pole
    /// geometry (the gate's separability measurements), not for
    /// re-measuring raw embeddings: [`Self::coordinates`] does its own
    /// centering.
    pub fn pole(&self, function: Function) -> &Vec<f32> {
        match function {
            Function::Thinking => &self.thinking,
            Function::Feeling => &self.feeling,
            Function::Sensing => &self.sensing,
            Function::Intuiting => &self.intuiting,
        }
    }

    fn pole_index(f: Function) -> usize {
        match f {
            Function::Thinking => 0,
            Function::Feeling => 1,
            Function::Sensing => 2,
            Function::Intuiting => 3,
        }
    }
}

/// One vector, centered on `mean` and renormalized (anisotropy removal).
/// A zero vector stays zero — no direction, so no manufactured signal;
/// a vector equal to the mean likewise (nothing left after subtraction).
fn centered(v: &[f32], mean: &[f32]) -> Vec<f32> {
    let mut out: Vec<f32> = v.iter().zip(mean.iter()).map(|(x, m)| x - m).collect();
    let norm = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return vec![0.0; out.len()];
    }
    for x in &mut out {
        *x /= norm;
    }
    out
}

/// Mean of the vectors, L2-normalized. `None` when empty or the mean
/// degenerates to zero length (no direction to normalize).
fn centroid(vectors: &[Vec<f32>]) -> Option<Vec<f32>> {
    if vectors.is_empty() {
        return None;
    }
    let dim = vectors[0].len();
    let mut mean = vec![0f32; dim];
    for v in vectors {
        if v.len() != dim {
            return None;
        }
        for (m, x) in mean.iter_mut().zip(v.iter()) {
            *m += x;
        }
    }
    #[allow(clippy::cast_precision_loss)] // exemplar counts stay far below 2^24
    let n = vectors.len() as f32;
    for m in &mut mean {
        *m /= n;
    }
    let norm = mean.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 || !norm.is_finite() {
        return None;
    }
    for m in &mut mean {
        *m /= norm;
    }
    Some(mean)
}

/// Cosine similarity; zero-norm inputs yield 0.0 (no signal, not NaN).
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// The gate (Task 22): pure geometry proceeds only if the poles
/// separate. Opposite-pole centroid cosine must be at or below this on
/// both axes.
pub const GATE_OPPOSITE_COSINE_MAX: f32 = 0.75;

/// The gate: per pole, the fraction of held-out exemplars whose
/// coordinate sign places them in their own pole must be at or above
/// this.
pub const GATE_DISCRIMINABILITY_MIN: f32 = 0.8;

/// The gate's measurements of a corpus: do the poles actually separate
/// in embedding space? Measured from the same exemplar evidence
/// [`AxisFrame::from_exemplars`] consumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Separability {
    /// Opposite-pole centroid cosine per axis:
    /// `[judgment (T·F), perception (S·N)]`.
    pub opposite_cosine: [f32; 2],
    /// Cross-axis centroid cosines, `[T·S, T·N, F·S, F·N]` — diagnostic,
    /// not gated.
    pub cross_cosine: [f32; 4],
    /// Mean cos(exemplar, own full-frame pole) per function, in
    /// [`Function::ALL`] order.
    pub coherence: [f32; 4],
    /// Leave-one-out discriminability per function: the fraction of
    /// exemplars whose own-axis coordinate sign — measured against a
    /// frame built without that exemplar — places them in their own
    /// pole. Thinking and sensing are the positive poles; feeling and
    /// intuiting the negative ones.
    pub discriminability: [f32; 4],
}

impl fmt::Display for Separability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "separability report")?;
        writeln!(
            f,
            "  opposite-pole cosine   judgment (T·F) {:.3}   perception (S·N) {:.3}   (max {})",
            self.opposite_cosine[0], self.opposite_cosine[1], GATE_OPPOSITE_COSINE_MAX
        )?;
        writeln!(
            f,
            "  cross-axis cosine      T·S {:.3}  T·N {:.3}  F·S {:.3}  F·N {:.3}",
            self.cross_cosine[0], self.cross_cosine[1], self.cross_cosine[2], self.cross_cosine[3]
        )?;
        for (i, func) in Function::ALL.iter().enumerate() {
            writeln!(
                f,
                "  {:<9} coherence {:.3}   leave-one-out {:.2}   (min {})",
                func.as_str(),
                self.coherence[i],
                self.discriminability[i],
                GATE_DISCRIMINABILITY_MIN
            )?;
        }
        writeln!(f, "  gate: {}", if self.passes() { "PASS" } else { "FAIL" })
    }
}

impl Separability {
    /// The gate verdict: both axes' opposite poles far enough apart, and
    /// every pole holding its own at leave-one-out.
    pub fn passes(&self) -> bool {
        self.opposite_cosine
            .iter()
            .all(|c| *c <= GATE_OPPOSITE_COSINE_MAX)
            && self
                .discriminability
                .iter()
                .all(|d| *d >= GATE_DISCRIMINABILITY_MIN)
    }
}

/// Measures a corpus's separability. `None` when no frame can be built
/// from the evidence (missing pole, inconsistent widths) — the gate
/// cannot even be run.
pub fn separability(embeddings: &[(Function, Vec<f32>)]) -> Option<Separability> {
    let frame = AxisFrame::from_exemplars(embeddings)?;
    let opposite_cosine = [
        cosine(
            frame.pole(Function::Thinking),
            frame.pole(Function::Feeling),
        ),
        cosine(
            frame.pole(Function::Sensing),
            frame.pole(Function::Intuiting),
        ),
    ];
    let cross_cosine = [
        cosine(
            frame.pole(Function::Thinking),
            frame.pole(Function::Sensing),
        ),
        cosine(
            frame.pole(Function::Thinking),
            frame.pole(Function::Intuiting),
        ),
        cosine(frame.pole(Function::Feeling), frame.pole(Function::Sensing)),
        cosine(
            frame.pole(Function::Feeling),
            frame.pole(Function::Intuiting),
        ),
    ];

    let mut coherence = [0f32; 4];
    let mut discriminability = [0f32; 4];
    for (i, f) in Function::ALL.iter().enumerate() {
        let members: Vec<usize> = embeddings
            .iter()
            .enumerate()
            .filter(|(_, (g, _))| g == f)
            .map(|(k, _)| k)
            .collect();
        if members.is_empty() {
            return None; // from_exemplars already guarantees this cannot happen
        }

        #[allow(clippy::cast_precision_loss)] // exemplar counts stay far below 2^24
        let n = members.len() as f32;
        coherence[i] = members
            .iter()
            .map(|&k| cosine(&embeddings[k].1, frame.pole(*f)))
            .sum::<f32>()
            / n;

        // Leave-one-out: each exemplar is judged by a frame that never
        // saw it. A frame that cannot be built without the exemplar
        // fails that exemplar — the conservative count.
        let correct = members
            .iter()
            .filter(|&&k| {
                let without: Vec<(Function, Vec<f32>)> = embeddings
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != k)
                    .map(|(_, (g, v))| (*g, v.clone()))
                    .collect();
                let Some(loo) = AxisFrame::from_exemplars(&without) else {
                    return false;
                };
                let (judgment, perception) = loo.coordinates(&embeddings[k].1);
                let coord = if f.axis() == Axis::Judgment {
                    judgment
                } else {
                    perception
                };
                let positive_pole = matches!(f, Function::Thinking | Function::Sensing);
                if positive_pole {
                    coord > 0.0
                } else {
                    coord < 0.0
                }
            })
            .count();
        #[allow(clippy::cast_precision_loss)] // exemplar counts stay far below 2^24
        let hits = correct as f32;
        discriminability[i] = hits / n;
    }

    Some(Separability {
        opposite_cosine,
        cross_cosine,
        coherence,
        discriminability,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../prompts/anchors-v1.md");

    #[test]
    fn function_strings_roundtrip_and_map_to_axes() {
        for f in Function::ALL {
            assert_eq!(Function::parse(f.as_str()), Some(f));
        }
        assert_eq!(Function::Thinking.axis(), Axis::Judgment);
        assert_eq!(Function::Feeling.axis(), Axis::Judgment);
        assert_eq!(Function::Sensing.axis(), Axis::Perception);
        assert_eq!(Function::Intuiting.axis(), Axis::Perception);
        assert_eq!(Function::Thinking.opposite(), Function::Feeling);
        assert_eq!(Function::Sensing.opposite(), Function::Intuiting);
        assert_eq!(Function::parse("valor"), None);
    }

    #[test]
    fn real_corpus_parses_four_poles_ten_each() {
        let exemplars = parse_corpus(CORPUS);
        for f in Function::ALL {
            assert_eq!(
                exemplars.iter().filter(|e| e.function == f).count(),
                10,
                "pole {} lost or gained exemplars",
                f.as_str()
            );
        }
        assert_eq!(exemplars.len(), 40);
    }

    #[test]
    fn parse_corpus_ignores_prose_and_unknown_headings() {
        let md = "# Title\n\nprose before any pole\n\n## thinking\n- one\n- two\n\n\
                  ## sencing\n- typo pole's text\n\n## feeling\n- three\n\nnot a bullet";
        let ex = parse_corpus(md);
        assert_eq!(ex.len(), 3);
        assert_eq!(ex[0].function, Function::Thinking);
        assert_eq!(ex[0].text, "one");
        assert_eq!(ex[2].function, Function::Feeling);
    }

    /// Orthogonal synthetic poles: each near a basis vector in 4-dim
    /// space. Coordinates must land with the right signs and near-zero
    /// cross-axis leakage.
    fn orthogonal_frame() -> AxisFrame {
        let e = |i: usize| -> Vec<f32> {
            let mut v = vec![0.05f32; 4];
            v[i] = 1.0;
            v
        };
        AxisFrame::from_exemplars(&[
            (Function::Thinking, e(0)),
            (Function::Thinking, e(0)),
            (Function::Feeling, e(1)),
            (Function::Feeling, e(1)),
            (Function::Sensing, e(2)),
            (Function::Sensing, e(2)),
            (Function::Intuiting, e(3)),
            (Function::Intuiting, e(3)),
        ])
        .expect("all poles present")
    }

    #[test]
    fn coordinates_sign_the_axes_correctly() {
        let frame = orthogonal_frame();

        let thinking_ward = vec![1.0f32, 0.0, 0.0, 0.0];
        let (j, p) = frame.coordinates(&thinking_ward);
        assert!(j > 0.5, "thinking-ward judgment coordinate, got {j}");
        assert!(p.abs() < 0.1, "no cross-axis leakage, got {p}");

        let feeling_ward = vec![0.0f32, 1.0, 0.0, 0.0];
        let (j, _) = frame.coordinates(&feeling_ward);
        assert!(j < -0.5, "feeling-ward is negative judgment, got {j}");

        let intuiting_ward = vec![0.0f32, 0.0, 0.0, 1.0];
        let (_, p) = frame.coordinates(&intuiting_ward);
        assert!(p < -0.5, "intuiting-ward is negative perception, got {p}");
    }

    #[test]
    fn frame_requires_every_pole_and_consistent_width() {
        assert!(AxisFrame::from_exemplars(&[(Function::Thinking, vec![1.0, 0.0])]).is_none());
        let mixed = AxisFrame::from_exemplars(&[
            (Function::Thinking, vec![1.0, 0.0]),
            (Function::Feeling, vec![0.0, 1.0, 0.0]),
        ]);
        assert!(mixed.is_none(), "width disagreement is no frame");
    }

    #[test]
    fn centroid_normalizes_and_rejects_degenerate_input() {
        let a = vec![3.0f32, 4.0];
        let b = vec![6.0f32, 8.0];
        let c = centroid(&[a, b]).expect("valid input");
        // mean is (4.5, 6), normalized toward (0.6, 0.8)
        assert!((c[0] - 0.6).abs() < 1e-5, "got {}", c[0]);
        assert!((c[1] - 0.8).abs() < 1e-5, "got {}", c[1]);

        let zero = vec![0.0f32, 0.0];
        assert!(
            centroid(&[zero.clone(), zero]).is_none(),
            "no direction to normalize"
        );
        assert!(centroid(&[]).is_none());
    }

    #[test]
    fn zero_embedding_yields_zero_coordinates_not_nan() {
        let frame = orthogonal_frame();
        let (j, p) = frame.coordinates(&[0.0f32; 4]);
        assert_eq!((j, p), (0.0, 0.0));
    }

    #[test]
    fn width_mismatch_yields_zero_coordinates() {
        let frame = orthogonal_frame();
        let (j, p) = frame.coordinates(&[1.0f32, 0.0, 0.0]);
        assert_eq!((j, p), (0.0, 0.0));
    }

    #[test]
    fn opposite_poles_face_away_after_centering() {
        let frame = orthogonal_frame();
        // In centered space the gate measures pole-vs-pole cosine:
        // near-orthogonal synthetic poles must sit well apart.
        let tf = cosine(
            frame.pole(Function::Thinking),
            frame.pole(Function::Feeling),
        );
        let sn = cosine(
            frame.pole(Function::Sensing),
            frame.pole(Function::Intuiting),
        );
        assert!(tf < 0.2, "T·F too close: {tf}");
        assert!(sn < 0.2, "S·N too close: {sn}");
    }
}
