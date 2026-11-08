//! Node-table DDL for the memory layer.
//!
//! The embedding column's width must match the live embedding model —
//! currently EmbeddingGemma-2 on the local `llama.cpp` server, whose
//! vectors are [`EMBEDDING_DIM`] wide. Because that width is a fact
//! about the outside world, the DDLs here are templates rendered from
//! the constant instead of static strings: one source of truth, no
//! literal dims to drift out of sync.

/// The width of every `FLOAT[dim]` embedding column — and the vector
/// dimension the embedding endpoint must produce.
pub const EMBEDDING_DIM: usize = 768;

// Raw user input -- the ground truth. Everything else is derived from notes.
const NOTE_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Note(
    id                STRING PRIMARY KEY,
    content           STRING,
    content_embedding FLOAT[{dim}],
    created_at        TIMESTAMP,
    updated_at        TIMESTAMP,
    layer             INT16 DEFAULT 0,
    context           STRING
)
";

const ENTITY_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Entity(
    id              STRING PRIMARY KEY,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[{dim}],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 1,
    context         STRING
)
";

const TIME_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Time(
    id              STRING PRIMARY KEY,
    granularity     STRING,
    starts_at       TIMESTAMP,
    ends_at         TIMESTAMP,
    label           STRING,
    label_resolved  STRING,
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 1,
    context         STRING
)
";

// Examples include, 'future', 'soon', 'never'. Let the agent anchor events to fuzzy temporal events
const ABSTRACT_TIME_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS AbstractTime(
    id              STRING PRIMARY KEY,
    semantics       STRING,
    label           STRING,
    label_resolved  STRING,
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 1,
    context         STRING
)
";

//A fact like 'a dog belongs to the k9 family' would have predicate "belongs_to", certainty: 1.0,
//source: 'general_knowledge'.
const FACT_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Fact(
    id              STRING PRIMARY KEY,
    predicate       STRING,
    certainty       DOUBLE DEFAULT 1.0,
    source          STRING,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[{dim}],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 2,
    context         STRING
)
";

// the status field captures, the state of event, 'occured', 'planned', 'cancelled', 'potential',
// the is_going marks in progress
const EVENT_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Event(
    id              STRING PRIMARY KEY,
    predicate       STRING,
    certainty       DOUBLE DEFAULT 1.0,
    source          STRING,
    status          STRING DEFAULT 'occurred',
    is_ongoing      BOOLEAN DEFAULT FALSE,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[{dim}],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 3,
    context         STRING
)
";

//Memory - experiences worth keeping! - significance -- why this matters!..
//emotions -- the emotional coloring ['excitement', 'vulnerable','determined', 'hope',
//'practical','anxiety']...
//reflection -- 'based on the Jungian and wunicorm evolving system of opposites, ie what value do I
//get from this, what can I intuite from this, where am I thinking this will go, what do my
//feelings tell me.
const MEMORY_T: &str = r"
CREATE NODE TABLE IF NOT EXISTS Memory(
    id              STRING PRIMARY KEY,
    predicate       STRING,
    certainty       DOUBLE DEFAULT 1.0,
    source          STRING,
    status          STRING DEFAULT 'occurred',
    is_ongoing      BOOLEAN DEFAULT FALSE,
    significance    STRING,
    emotions        STRING[],
    reflection      STRING,
    label           STRING,
    label_resolved  STRING,
    label_embedding FLOAT[{dim}],
    learned_at      TIMESTAMP,
    expire_at       TIMESTAMP,
    created_at      TIMESTAMP,
    updated_at      TIMESTAMP,
    layer           INT16 DEFAULT 4,
    context         STRING
)
";

/// The entity-table DDLs, rendered from [`EMBEDDING_DIM`]. Same
/// declaration order as ever: Note, Entity, Time, `AbstractTime`,
/// Fact, Event, Memory.
pub fn entity_node_ddls() -> Vec<String> {
    [
        NOTE_T,
        ENTITY_T,
        TIME_T,
        ABSTRACT_TIME_T,
        FACT_T,
        EVENT_T,
        MEMORY_T,
    ]
    .iter()
    .map(|t| t.replace("{dim}", &EMBEDDING_DIM.to_string()))
    .collect()
}
