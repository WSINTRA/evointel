# Anchor corpus v1 — the four poles

The Jungian function axes, as geometry. Two bipolar pairs:

- **judgment** — `thinking ↔ feeling`: how a thing is *evaluated*
  (impersonal consequence ↔ personal worth)
- **perception** — `sensing ↔ intuiting`: how a thing is *taken in*
  (concrete presence ↔ pattern and possibility)

Each pole is seeded from the exemplar texts below. A pole's anchor is the
centroid (mean, L2-normalized) of its exemplars' embeddings **after
corpus-mean centering** — anisotropy removal, the standard all-but-the-top
postprocessing — computed at seed time and stored as `FunctionAnchor`
rows. A raw embedding's coordinate on an axis is
`cos(x_c, own_pole) − cos(x_c, opposite_pole)`, where `x_c` is the
embedding centered on the same corpus mean.

**Why centering (the gate's own finding):** EmbeddingGemma-2's raw space
is anisotropic — every pole centroid sits 0.93–0.95 cosine from every
other, and the raw corpus *failed* the gate (judgment T·F 0.940,
perception S·N 0.942). After centering, the poles face away from each
other and the corpus passes with margin (v1, live server, Task 22):
T·F −0.351, S·N −0.266; leave-one-out 1.00 thinking, 1.00 feeling,
1.00 sensing, 0.90 intuiting. The mean is computed from the exemplars,
never trained — still pure geometry, recomputable from the stored anchor
rows alone.

**Calibration note:** the first draft of this corpus failed the gate on
seven exemplars that contaminated their own register — feeling texts
that invoked numbers and efficiency, a sensing text that reasoned
diagnostically ("so the click isn't…"), intuiting texts that touched
mood and memory. The lesson, now a rule for future revisions: an
exemplar **does** its function; it never contrasts against, reasons
about, or narrates a neighboring one.

The exemplars are written in the pole's *register* — a thinking-pole text
does thinking; it never says "this is thinking." First person, short,
apprehension-shaped: how a mind actually moves when the function is
engaged.

**Provenance:** LLM-drafted, human-reviewed, then frozen. This file is
the artifact of record. A change to the corpus is a v2 corpus and a
re-seed (the old `FunctionAnchor` rows are deactivated, never deleted —
anchor history is preserved; the auditor is audited).

**The gate** (`tests/anchor_eval.rs`): the corpus must separate before
pure geometry is trusted — opposite-pole centroid cosine at or below
`GATE_OPPOSITE_COSINE_MAX` (0.75) on both axes, and leave-one-out
discriminability at or above `GATE_DISCRIMINABILITY_MIN` (0.8) for every
pole.

## thinking

- If the deadline moves to Friday, the critical path shifts to the supplier, and everything downstream re-plans around it.
- Once I subtract the setup hour from both options, the price difference inverts.
- I keep turning the argument over, checking each step for what it assumes.
- Two of these three claims contradict, so at most one of them survives.
- The rule is fine; the exception would swallow the rule, so the exception goes.
- I want to see the derivation, not just the conclusion it arrives at.
- Where does this break if ten times as many people show up?
- That conclusion only follows if we accept the premise nobody stated.
- I ran the timeline backward from the launch date and the training slot disappears.
- It works in principle, so now I'm looking for the failure mode I haven't found yet.

## feeling

- I keep coming back to how that message landed — it mattered more than the meeting it interrupted.
- Everyone said take the offer; the part of me that knows what I'd be leaving kept saying no.
- I chose the slower way because the faster way left everyone feeling like traffic.
- What tipped me wasn't the argument — it was that she trusted me with it.
- I stayed longer than I needed to, because leaving would have felt like taking something back.
- It felt like a small betrayal, filed under logistics.
- I can defend the decision and still regret its cost to the person who wanted the other outcome.
- The gift was wrong in every practical way and exactly right anyway.
- Some things I keep not because they're useful but because letting go would say something about me I don't want said.
- That compliment sat with me all day, and I noticed I'd been carrying the criticism for weeks.

## sensing

- The meeting room is on three, second door past the elevator, and the radiator clunks every twenty minutes or so.
- New battery in this morning; the click is still there, same rhythm as before.
- I wrote down the street name because the map kept redrawing itself.
- The bread's darker on the bottom than the last batch; the oven runs hot on the left.
- Rain started around four — proper rain, the kind that pools at the curb.
- I can still place the smell of that hallway: floor wax and old smoke.
- The receipt says Tuesday, so it was Tuesday.
- There were four of them at the corner table, jackets still wet.
- My hands remember the chord even when I've forgotten I learned it.
- The page count, the word count, the time it took — I keep the log because the log doesn't editorialize.

## intuiting

- This is the third time this month the same shape has appeared wearing different clothes — something is forming.
- I can't defend it yet, but the whole thing reads like a preamble to something else.
- The interesting move isn't in this position; it's the position it quietly sets up three moves later.
- Everyone's answering the question nobody is asking — the real question is two questions upstream.
- I can feel the next chapter of this forming before anything in it is true yet.
- The pieces don't connect yet, but they keep leaning toward each other.
- What if the constraint isn't the constraint — what if it's the door?
- I knew the room had turned before anyone said anything — the pattern gave it away two beats early.
- The pattern wants completing; the missing piece is shaped like the thing I set aside last spring.
- Something is trying to happen here, and my job is to stay out of its way long enough to see what.
