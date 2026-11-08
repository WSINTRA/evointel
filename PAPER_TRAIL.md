# Paper Trail

Concrete decisions made during planning and development. Append-only — no entries are deleted, superseded entries get a note.

---

## D-001: No app-level encryption

**Date:** 2026-10-27

**Decision:** Remove client-side zero-knowledge encryption (password, seed phrase, WebCrypto KDF, AES-GCM) from the design. No application-level encryption of note content at rest or in transit.

**Context:** The original plan (§4) specified a crypto-wallet-style key model: password → KDF → key derived in the browser, all content stored as ciphertext, server never sees the key. This was designed for a threat model where the server is untrusted.

**Reasoning:**
- The daemon IS the server. It needs plaintext to process notes (sorter, graph, insights, LLM calls).
- Client-side zero-knowledge is architecturally incompatible with a background processing daemon — the daemon would need the key to do its job.
- The actual threat ("Mac stolen") is already covered by FileVault (full-disk encryption).
- Tailscale covers in-transit encryption between devices.
- If a non-local LLM is configured, data leaves the machine in plaintext regardless of at-rest encryption — redaction is the meaningful control there.
- Single-user, local-first, no multi-tenant server to protect against.
- Adding encryption introduces complexity (key management, rotation, performance) without a matching threat.

**What stays:**
- FileVault (OS-level at-rest encryption)
- Tailscale (encrypted transport)
- Redaction before LLM calls (the real data-exit boundary)
- Local LLMs as the default (no data leaves the machine at all)

**Revisit trigger:** If the product grows into a distributed or cloud-hosted service, the trust model changes and application-level encryption becomes necessary.

**Files affected:** `plan.md` (§1, §4, §5, §7, §11, §12, Appendix A), `index.html` (sections 04, 05, 06, 08, 12, 13)

---

## D-002: A two-point Jungian function space in the memory graph

**Date:** 2026-11-08

**Decision:** The Jungian functions move from prompt language into the graph itself. Two bipolar axes — judgment (thinking ↔ feeling) and perception (sensing ↔ intuiting) — built from contrastive anchor centroids seeded from a frozen exemplar corpus. Every note projects to a signed 2-dim coordinate; every dream's choices project Wunicorm into the same frame, as a damped core plus a rigidity-gated compensation component targeting dyad balance. A deterministic self pass in the dream cycle measures, persists, and grounds all of it; the sermon speaks from it in movement language only.

**Context:** The v1 SELF was a closed loop with no outside anchor (see D-001's sibling row in §1: "no revision without observation finally has real observations"). The function stack lived only in sermon prompt adjectives. The gate (live, EmbeddingGemma-2) produced the decisive numbers: the **raw** embedding space failed — opposite-pole centroid cosine 0.940 (judgment) / 0.942 (perception), every pole 0.93–0.95 from every other (anisotropy); leave-one-out 0.70 for feeling. **Corpus-mean centering** (all-but-the-top anisotropy removal — computed, never trained) rescued pure geometry: opposite poles at −0.351 / −0.266, leave-one-out 1.00 / 1.00 / 1.00 / 0.90. The corpus itself needed one calibration pass: the first draft failed on seven cross-register exemplars (feeling texts invoking numbers and efficiency, a sensing text reasoning diagnostically, intuiting texts touching mood and memory) — the rule that fell out: *an exemplar does its function; it never contrasts against, reasons about, or narrates a neighboring one.* A second empirical pin: the straightness (rigidity) index requires **≥3 windows** — two points are always a straight line, and one swing is movement, not stuckness.

**Reasoning:**
- **Two points, one frame, asymmetric evidence.** U (the user) from notes — human-authored ground truth. W (Wunicorm) from choices only — kept/dropped/emphasized per function — because personality is in behavior, and choice-loadings are register-robust where prose is not.
- **Rigidity, not direction.** Compensation is gated on straightness of trajectory, never on the stance's direction — a creative sprint is rightly intuition-heavy; medicine scales to stuckness. (Engineered compensation, per the design conversation.)
- **Damped prescription.** Compensation strength starts small (k=0.25, θ=0.7) and the core damping (α=0.9) is the hysteresis that makes personality rather than mood — months, not windows. All constants in one documented block, logged per run.
- **The auditor is audited.** Nothing compensates W; a self defined by counter-position could radicalize into its own stuckness. The same rigidity rule reads W's core; anchor re-seeds deactivate rather than delete (history preserved); the compensation parameters are themselves subject to no-revision-without-observation.
- **No claimed effects.** The sermon never attributes the user's movement to the system — no flattery, promoted to architecture. Numbers are stripped at material construction, never entrusted to the prompt.
- **The discarded is data.** A `DreamRun` ledger records considered vs kept per function; drops dilute W's confidence (δ=0.5), never push its position.
- **Derived, never authoritative.** Positions are computed from notes + recorded choices; every positional claim traces to nodes via `Grounds` edges; SelfStates rebuild identically from the evidence (proven in the full eval). Unseeded anchors are a quiet skip — the self layer is opt-in, and a pre-self DB dreams exactly as before.

**What stays:**
- The four pillars, the drift machinery, the dreamer-as-only-writer contract, messages outside the graph with citations — untouched.
- Pure geometry as the derivation method — the learned-probe fallback was designed but not needed; centering sufficed.

**Revisit triggers:**
- Any embedding-model change re-runs the live gate (`WUNICORM_LLM_LIVE=1 cargo test --test anchor_eval`); separability below threshold → the probe fallback.
- Sermon self-language review after the first weeks of real dreams (is movement-language useful, or noise?).
- W's own core stuckness monitoring once months of trajectory exist (the auditor-audited rule, made operational).

**Files affected:** `memory-proto/` (migration batches v2-v3: `FunctionAnchor`, `SelfState`, `DreamRun`, `Grounds`, `SELF_TRAJECTORY`, Note/Memory columns; `src/axes.rs`, `src/queries/psyche.rs`, the self pass in `src/dreamer.rs`, `prompts/anchors-v1.md` + extract/sermon prompt updates, the gate in `tests/anchor_eval.rs`, AGILE.md Tasks 20-31), `plan.md` (§6)
