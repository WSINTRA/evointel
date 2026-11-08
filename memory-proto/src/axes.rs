//! The function axes — Jung's two bipolar pairs as geometry.
//!
//! - **judgment**: thinking ↔ feeling — how a thing is evaluated
//!   (impersonal consequence ↔ personal worth)
//! - **perception**: sensing ↔ intuiting — how a thing is taken in
//!   (concrete presence ↔ pattern and possibility)
//!
//! A pole's anchor is the centroid of its exemplar embeddings (mean,
//! L2-normalized); a text's coordinate on an axis is
//! `cos(x, own_pole) − cos(x, opposite_pole)` — signed, so positive
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

/// The two axes as anchor centroids. `None` when a pole has no
/// exemplars — the caller's cue to degrade, not to invent a frame.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisFrame {
    thinking: Vec<f32>,
    feeling: Vec<f32>,
    sensing: Vec<f32>,
    intuiting: Vec<f32>,
}

impl AxisFrame {
    /// Builds the frame from exemplar embeddings: per-pole centroid
    /// (mean, L2-normalized). `None` if any pole is empty or the
    /// vectors disagree in width.
    pub fn from_exemplars(embeddings: &[(Function, Vec<f32>)]) -> Option<Self> {
        let mut by_pole: [Vec<&Vec<f32>>; 4] = [vec![], vec![], vec![], vec![]];
        let mut dim = None;
        for (f, v) in embeddings {
            if v.is_empty() {
                return None;
            }
            match dim {
                None => dim = Some(v.len()),
                Some(d) if d != v.len() => return None,
                Some(_) => {}
            }
            by_pole[Self::pole_index(*f)].push(v);
        }
        let [t, f, s, n] = by_pole;
        Some(Self {
            thinking: centroid(&t)?,
            feeling: centroid(&f)?,
            sensing: centroid(&s)?,
            intuiting: centroid(&n)?,
        })
    }

    /// A pole's coordinate pair for an embedding: `(judgment, perception)`.
    /// Positive judgment is thinking-ward; positive perception is
    /// sensing-ward. Zero-norm vectors yield `(0.0, 0.0)` — no signal,
    /// not NaN.
    pub fn coordinates(&self, embedding: &[f32]) -> (f32, f32) {
        let judgment = cosine(embedding, &self.thinking) - cosine(embedding, &self.feeling);
        let perception = cosine(embedding, &self.sensing) - cosine(embedding, &self.intuiting);
        (judgment, perception)
    }

    /// The anchor embedding for a pole (centroids are public to the
    /// gate eval, which measures separability between them).
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

/// Mean of the vectors, L2-normalized. `None` when empty or the mean
/// degenerates to zero length (no direction to normalize).
fn centroid(vectors: &[&Vec<f32>]) -> Option<Vec<f32>> {
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
        let c = centroid(&[&a, &b]).expect("valid input");
        // mean is (4.5, 6), normalized toward (0.6, 0.8)
        assert!((c[0] - 0.6).abs() < 1e-5, "got {}", c[0]);
        assert!((c[1] - 0.8).abs() < 1e-5, "got {}", c[1]);

        let zero = vec![0.0f32, 0.0];
        assert!(
            centroid(&[&zero, &zero]).is_none(),
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
    fn poles_are_exposed_for_the_gate() {
        let frame = orthogonal_frame();
        // The thinking pole centroid must point thinking-ward: its own
        // coordinate is positive on judgment.
        let (j, _) = frame.coordinates(frame.pole(Function::Thinking));
        assert!(j > 0.5);
    }
}
