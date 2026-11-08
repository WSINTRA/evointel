// Space-time relation tables

// Temporal relations
pub const CREATE_NODE_TABLE_BEFORE: &str = r"
CREATE NODE TABLE IF NOT EXISTS Before(
    id               STRING PRIMARY KEY,
    gap_duration     STRING,
    confidence       DOUBLE DEFAULT 1.0,
    label            STRING,
    label_resolved   STRING,
    learned_at       TIMESTAMP,
    expire_at        TIMESTAMP,
    created_at       TIMESTAMP,
    updated_at       TIMESTAMP,
    layer            INT16 DEFAULT 0,
    context          STRING
)";
pub const CREATE_NODE_TABLE_AFTER: &str = r"
CREATE NODE TABLE IF NOT EXISTS After(
    id               STRING PRIMARY KEY,
    gap_duration     STRING,
    confidence       DOUBLE DEFAULT 1.0,
    label            STRING,
    label_resolved   STRING,
    learned_at       TIMESTAMP,
    expire_at        TIMESTAMP,
    created_at       TIMESTAMP,
    updated_at       TIMESTAMP,
    layer            INT16 DEFAULT 0,
    context          STRING
)";
pub const CREATE_NODE_TABLE_DURING: &str = r"
CREATE NODE TABLE IF NOT EXISTS During(
    id               STRING PRIMARY KEY,
    overlap_type     STRING DEFAULT 'full',
    confidence       DOUBLE DEFAULT 1.0,
    label            STRING,
    label_resolved   STRING,
    learned_at       TIMESTAMP,
    expire_at        TIMESTAMP,
    created_at       TIMESTAMP,
    updated_at       TIMESTAMP,
    layer            INT16 DEFAULT 0,
    context          STRING
)";
// Validity relations
pub const CREATE_NODE_TABLE_VALID_FROM: &str = r"
CREATE NODE TABLE IF NOT EXISTS ValidFrom(
    id               STRING PRIMARY KEY,
    precision        STRING DEFAULT 'exact',
    confidence       DOUBLE DEFAULT 1.0,
    label            STRING,
    label_resolved   STRING,
    learned_at       TIMESTAMP,
    expire_at        TIMESTAMP,
    created_at       TIMESTAMP,
    updated_at       TIMESTAMP,
    layer            INT16 DEFAULT 0,
    context          STRING
)";
pub const CREATE_NODE_TABLE_VALID_TO: &str = r"
CREATE NODE TABLE IF NOT EXISTS ValidTo(
    id               STRING PRIMARY KEY,
    precision        STRING DEFAULT 'exact',
    confidence       DOUBLE DEFAULT 1.0,
    termination      STRING,
    label            STRING,
    label_resolved   STRING,
    learned_at       TIMESTAMP,
    expire_at        TIMESTAMP,
    created_at       TIMESTAMP,
    updated_at       TIMESTAMP,
    layer            INT16 DEFAULT 0,
    context          STRING
)";
pub const ALL_RELATION_DDLS: &[&str] = &[
    CREATE_NODE_TABLE_AFTER,
    CREATE_NODE_TABLE_BEFORE,
    CREATE_NODE_TABLE_DURING,
    CREATE_NODE_TABLE_VALID_TO,
    CREATE_NODE_TABLE_VALID_FROM,
];
