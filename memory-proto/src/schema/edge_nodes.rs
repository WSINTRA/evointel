pub const CREATE_EDGE_NODE_TABLE_CONTAINS: &str = r"
CREATE NODE TABLE IF NOT EXISTS Contains(
    id                 STRING PRIMARY KEY,
    containment_types  STRING,
    weight             DOUBLE DEFAULT 1.0,
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const CREATE_EDGE_NODE_TABLE_SIMILAR: &str = r"
CREATE NODE TABLE IF NOT EXISTS Similar(
    id                 STRING PRIMARY KEY,
    similarity         DOUBLE DEFAULT 0.0,
    sim_context        STRING,
    sim_method         STRING,
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const CREATE_EDGE_NODE_TABLE_HAS_PROPERTY: &str = r"
CREATE NODE TABLE IF NOT EXISTS HasProperty(
    id                 STRING PRIMARY KEY,
    property_name      STRING,
    property_value     STRING,
    prop_context       STRING,
    certainty          DOUBLE DEFAULT 1.0,
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const CREATE_EDGE_NODE_TABLE_LEADS_TO: &str = r"
CREATE NODE TABLE IF NOT EXISTS LeadsTo(
    id                 STRING PRIMARY KEY,
    probability        DOUBLE DEFAULT 1.0,
    strength           DOUBLE DEFAULT 1.0,
    mechanism          STRING,
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const CREATE_EDGE_NODE_TABLE_CAUSES: &str = r"
CREATE NODE TABLE IF NOT EXISTS Causes(
    id                 STRING PRIMARY KEY,
    probability        DOUBLE DEFAULT 1.0,
    strength           DOUBLE DEFAULT 1.0,
    mechanism          STRING,
    directness         STRING DEFAULT 'direct',
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const CREATE_EDGE_NODE_TABLE_PREVENTS: &str = r"
CREATE NODE TABLE IF NOT EXISTS Prevents(
    id                 STRING PRIMARY KEY,
    probability        DOUBLE DEFAULT 1.0,
    strength           DOUBLE DEFAULT 1.0,
    mechanism          STRING,
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const CREATE_EDGE_NODE_TABLE_BECAUSE_OF: &str = r"
CREATE NODE TABLE IF NOT EXISTS BecauseOf(
    id                 STRING PRIMARY KEY,
    probability        DOUBLE DEFAULT 1.0,
    strength           DOUBLE DEFAULT 1.0,
    explanation        STRING,
    label              STRING,
    label_resolved     STRING,
    learned_at         TIMESTAMP,
    expire_at          TIMESTAMP,
    created_at         TIMESTAMP,
    updated_at         TIMESTAMP,
    layer              INT16 DEFAULT 0,
    context            STRING
)
";

pub const ALL_EDGE_NODE_DDLS: &[&str] = &[
    CREATE_EDGE_NODE_TABLE_CONTAINS,
    CREATE_EDGE_NODE_TABLE_SIMILAR,
    CREATE_EDGE_NODE_TABLE_HAS_PROPERTY,
    CREATE_EDGE_NODE_TABLE_LEADS_TO,
    CREATE_EDGE_NODE_TABLE_CAUSES,
    CREATE_EDGE_NODE_TABLE_PREVENTS,
    CREATE_EDGE_NODE_TABLE_BECAUSE_OF,
];
