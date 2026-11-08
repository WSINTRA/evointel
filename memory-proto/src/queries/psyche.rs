//! Psyche reads — the self's positions, rigidity, and drift in
//! function space.
//!
//! The 2-dim mirror of `drift.rs`: where drift measures how a concept's
//! *neighborhood* moved in embedding space, these measure how the
//! *self* moved on the axes — U (the user) from measured notes, W
//! (Wunicorm) from its recorded choices. Positions are derived, never
//! authoritative: every positional claim traces through `Grounds` edges
//! to the nodes it came from.
//!
//! Unmeasurable is `None` and stuck is `0.0` — "can't tell" and
//! "didn't move" are different claims, the drift module's law.

use chrono::{DateTime, Utc};
use lbug::{Connection, Value};

use crate::axes::{
    Axis, AxisFrame, COMPENSATION_BASE, CORE_DAMPING, DROP_DILUTION, Function, RIGIDITY_THRESHOLD,
};
use crate::db::DbResult;

use super::fetch::time_to_chrono;

/// A point in function space: `(judgment, perception)`.
pub type Position = (f32, f32);

/// Per-axis displacement between two self positions — the function-space
/// drift reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drift2 {
    pub judgment: f32,
    pub perception: f32,
}

/// Wunicorm's decomposition after one update: the damped core, the
/// rigidity-gated compensation, and the position they sum to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decomposition {
    pub core: Position,
    pub comp: Position,
    pub position: Position,
}

// ─── Positions ───────────────────────────────────────────────────────

/// The frame the active anchors define — the axes the self pass
/// measures with. `None` when no anchors are seeded (or a pole is
/// empty): the caller's cue to degrade, not to invent geometry.
pub fn active_axis_frame(conn: &Connection) -> DbResult<Option<AxisFrame>> {
    let result = conn
        .query("MATCH (a:FunctionAnchor) WHERE a.active = true RETURN a.function, a.embedding;")?;
    let mut evidence: Vec<(Function, Vec<f32>)> = vec![];
    for row in result {
        let (Value::String(f), Value::Array(_, items)) = (&row[0], &row[1]) else {
            continue;
        };
        let Some(f) = Function::parse(f) else {
            continue;
        };
        let embedding: Vec<f32> = items
            .iter()
            .filter_map(|v| match v {
                Value::Float(x) => Some(*x),
                _ => None,
            })
            .collect();
        evidence.push((f, embedding));
    }
    Ok(AxisFrame::from_exemplars(&evidence))
}

/// U's position: the centroid of measured note coordinates whose
/// `created_at` falls in the window (notes bucket by `created_at` —
/// the drift module's convention). `None` when no measured notes fall
/// in the window — unmeasurable, not centered.
pub fn user_position(
    conn: &Connection,
    window: (DateTime<Utc>, DateTime<Utc>),
) -> DbResult<Option<Position>> {
    let result =
        conn.query("MATCH (n:Note) WHERE n.judgment_coord IS NOT NULL AND n.perception_coord IS NOT NULL RETURN n.created_at, n.judgment_coord, n.perception_coord;")?;
    let mut sum = (0f64, 0f64);
    let mut n = 0u64;
    for row in result {
        let Value::Timestamp(at) = &row[0] else {
            continue;
        };
        let at = time_to_chrono(at);
        if at >= window.0
            && at <= window.1
            && let (Value::Double(j), Value::Double(p)) = (&row[1], &row[2])
        {
            sum.0 += j;
            sum.1 += p;
            n += 1;
        }
    }
    if n == 0 {
        return Ok(None);
    }
    #[allow(clippy::cast_precision_loss)] // note counts stay far below 2^24
    let n = n as f64;
    // f64 column storage back to the f32 the axes speak — the same
    // precision the coordinates were written with.
    #[allow(clippy::cast_possible_truncation)]
    Ok(Some(((sum.0 / n) as f32, (sum.1 / n) as f32)))
}

/// W's observed position for one dream run, from its recorded choices:
/// per axis, `(own_pole_keeps − opposite_keeps) / (keeps + δ·drops)`.
/// Drops dilute — they shrink the position toward zero, never push it.
/// `None` when the run has no ledger row; an axis with no evidence at
/// all reads `0.0`.
pub fn wunicorm_observed(conn: &Connection, run_id: &str) -> DbResult<Option<Position>> {
    let mut stmt = conn.prepare(
        "MATCH (r:DreamRun {id: $id}) RETURN r.kept_t, r.kept_f, r.kept_s, r.kept_n, r.dropped_total;",
    )?;
    let result = conn.execute(&mut stmt, vec![("id", Value::String(run_id.to_string()))])?;
    let Some(row) = result.into_iter().next() else {
        return Ok(None);
    };
    let ints: Vec<i64> = row
        .into_iter()
        .map(|v| match v {
            Value::Int64(i) => i,
            _ => 0,
        })
        .collect();
    let (kept_t, kept_f, kept_s, kept_n, dropped) = (ints[0], ints[1], ints[2], ints[3], ints[4]);
    #[allow(clippy::cast_precision_loss)] // per-run choice counts stay tiny
    let dilution = DROP_DILUTION * dropped as f32;
    #[allow(clippy::cast_precision_loss)] // per-run choice counts stay tiny
    let judgment = {
        let num = (kept_t - kept_f) as f32;
        let denom = (kept_t + kept_f) as f32 + dilution;
        if denom == 0.0 { 0.0 } else { num / denom }
    };
    #[allow(clippy::cast_precision_loss)] // per-run choice counts stay tiny
    let perception = {
        let num = (kept_s - kept_n) as f32;
        let denom = (kept_s + kept_n) as f32 + dilution;
        if denom == 0.0 { 0.0 } else { num / denom }
    };
    Ok(Some((judgment, perception)))
}

// ─── The trajectory ──────────────────────────────────────────────────

/// One self state on the trajectory: its window and position.
struct TrajectoryPoint {
    window_start: DateTime<Utc>,
    position: Position,
}

/// The subject's recent trajectory, chronological, at most `span` long.
fn trajectory(conn: &Connection, subject: &str, span: usize) -> DbResult<Vec<TrajectoryPoint>> {
    let mut stmt = conn.prepare(
        "MATCH (s:SelfState {subject: $subject}) RETURN s.window_start, s.judgment_coord, s.perception_coord;",
    )?;
    let result = conn.execute(
        &mut stmt,
        vec![("subject", Value::String(subject.to_string()))],
    )?;
    let mut points: Vec<TrajectoryPoint> = result
        .into_iter()
        .filter_map(|row| {
            let (Value::Timestamp(at), Value::Double(j), Value::Double(p)) =
                (&row[0], &row[1], &row[2])
            else {
                return None;
            };
            #[allow(clippy::cast_possible_truncation)] // f64 coords came from f32
            Some(TrajectoryPoint {
                window_start: time_to_chrono(at),
                position: (*j as f32, *p as f32),
            })
        })
        .collect();
    points.sort_by_key(|p| p.window_start);
    if points.len() > span {
        points.drain(..points.len() - span);
    }
    Ok(points)
}

/// The straightness index on one axis over the trailing `span` self
/// states: net displacement ÷ path length, in `[0, 1]`. Near `1` is a
/// rigid line — the same stance held across windows; near `0` is
/// wandering. Fewer than three states → `0.0`: two points are always a
/// straight line, and one swing is movement, not stuckness.
pub fn rigidity(conn: &Connection, subject: &str, axis: Axis, span: usize) -> DbResult<f32> {
    let points = trajectory(conn, subject, span)?;
    let coords: Vec<f32> = points
        .iter()
        .map(|p| {
            if axis == Axis::Judgment {
                p.position.0
            } else {
                p.position.1
            }
        })
        .collect();
    Ok(straightness(&coords))
}

/// Pure straightness: `|last − first| ÷ Σ|consecutive steps|`.
fn straightness(coords: &[f32]) -> f32 {
    if coords.len() < 3 {
        // Two points are always a straight line — one swing is
        // movement, not stuckness. Rigidity is a claim that needs a
        // trajectory behind it.
        return 0.0;
    }
    let net = (coords[coords.len() - 1] - coords[0]).abs();
    let path: f32 = coords.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
    if path <= f32::EPSILON {
        return 0.0;
    }
    (net / path).clamp(0.0, 1.0)
}

/// Per-axis displacement between the subject's two most recent self
/// states — the function-space drift reading. `None` until the subject
/// has walked at least two windows.
pub fn function_drift(conn: &Connection, subject: &str) -> DbResult<Option<Drift2>> {
    let points = trajectory(conn, subject, 2)?;
    let (Some(from), Some(to)) = (points.first(), points.get(1)) else {
        return Ok(None);
    };
    Ok(Some(Drift2 {
        judgment: to.position.0 - from.position.0,
        perception: to.position.1 - from.position.1,
    }))
}

// ─── The W decomposition ─────────────────────────────────────────────

/// One update of Wunicorm's position, pure math: the damped core
/// absorbs this run's observed position; the fast component leans
/// against the user's current position, gated by rigidity — zero below
/// the threshold, never by direction. `k_eff` is the damper, so the
/// compensation is bounded by construction (`|comp| ≤ k·|user|`).
pub fn evolve_wunicorm(
    prev_core: Position,
    observed: Position,
    user: Position,
    rigidities: (f32, f32),
) -> Decomposition {
    let evolve_axis = |core: f32, observed: f32, user: f32, rigidity: f32| {
        let core = CORE_DAMPING * core + (1.0 - CORE_DAMPING) * observed;
        let k_eff = COMPENSATION_BASE * (rigidity - RIGIDITY_THRESHOLD).max(0.0);
        let comp = -k_eff * user;
        (core, comp)
    };
    let (core_j, comp_j) = evolve_axis(prev_core.0, observed.0, user.0, rigidities.0);
    let (core_p, comp_p) = evolve_axis(prev_core.1, observed.1, user.1, rigidities.1);
    Decomposition {
        core: (core_j, core_p),
        comp: (comp_j, comp_p),
        position: (core_j + comp_j, core_p + comp_p),
    }
}
