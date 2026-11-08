# Anchor corpus v1 — the four poles

The Jungian function axes, as geometry. Two bipolar pairs:

- **judgment** — `thinking ↔ feeling`: how a thing is *evaluated*
  (impersonal consequence ↔ personal worth)
- **perception** — `sensing ↔ intuiting`: how a thing is *taken in*
  (concrete presence ↔ pattern and possibility)

Each pole is seeded from the exemplar texts below. A pole's anchor is the
centroid (mean, L2-normalized) of its exemplars' embeddings, computed at
seed time and stored as `FunctionAnchor` rows. A text's coordinate on an
axis is `cos(x, own_pole) − cos(x, opposite_pole)`.

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
- The cheaper option wins once I subtract what the extra hour of setup actually costs.
- I keep turning the argument over, checking each step for what it assumes.
- Two of these three claims contradict, so at most one of them survives.
- The rule is fine; the exception would swallow the rule, so the exception goes.
- I want the numbers before I care about the pitch.
- Where does this break if ten times as many people show up?
- That conclusion only follows if we accept the premise nobody stated.
- I ran the timeline backward from the launch date and the training slot disappears.
- It works in principle, so now I'm looking for the failure mode I haven't found yet.

## feeling

- I keep coming back to how that message landed — it mattered more than the meeting it interrupted.
- The numbers say take the offer; something in me says this team is worth staying for.
- I don't want the efficient version if the efficient version is cold.
- What tipped me wasn't the argument — it was that she trusted me with it.
- Losing the ritual would save an hour a week and cost the week its shape.
- It felt like a small betrayal, filed under logistics.
- I can defend the decision and still regret its cost to the person who wanted the other outcome.
- The gift was wrong in every practical way and exactly right anyway.
- Some things I keep not because they're useful but because letting go would say something about me I don't want said.
- That compliment sat with me all day, and I noticed I'd been carrying the criticism for weeks.

## sensing

- The meeting room is on three, second door past the elevator, and the radiator clunks every twenty minutes or so.
- New battery in, and it still clicks — so the click isn't the battery.
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
- It feels like the early days of a story I've seen before, and I mostly remember how that story ended.
- The pieces don't connect yet, but they keep leaning toward each other.
- What if the constraint isn't the constraint — what if it's the door?
- I read the room's mood two beats before anyone said anything.
- The pattern wants completing; the missing piece is shaped like the thing I set aside last spring.
- Something is trying to happen here, and my job is to stay out of its way long enough to see what.
