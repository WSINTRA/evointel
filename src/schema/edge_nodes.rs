// Edge node tables (layer 0) - to be populated.
pub const CREATE_EDGE_NODE_TABLE_CONTAINS: &str = r"
   id                 STRING PRIMARY KEY,
   containment_types  STRING,
   weight             DOUBLE DEFAULT 1.0,
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";

pub const CREATE_EDGE_NODE_TABLE_SIMILAR: &str = r"
   id                 STRING PRIMARY KEY,
   similarity         DOUBLE DEFAULT 0.0, 
   sim_context        STRING,
   sim_method         STRING 
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";

pub const CREATE_EDGE_NODE_TABLE_HAS_PROPERTY: &str = r"
   id                 STRING PRIMARY KEY,
   property_name      STRING,
   property_value     STRING,
   prop_context       STRING,
   certainty          DOUBLE DEFAULT 1.0,
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";

pub const CREATE_EDGE_NODE_TABLE_LEADS_TO: &str = r"
   id                 STRING PRIMARY KEY,
   probability        DOUBLE DEFAULT 1.0,
   strength           DOUBLE DEFAULT 1.0,
   mechanism          STRING,
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";

pub const CREATE_EDGE_NODE_TABLE_CAUSES: &str = r"
   id                 STRING PRIMARY KEY,
   probability        DOUBLE DEFAULT 1.0,
   strength           DOUBLE DEFAULT 1.0,
   mechanism          STRING,
   directness         STRING DEFAULT 'direct',
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";
pub const CREATE_EDGE_NODE_TABLE_PREVENTS: &str = r"
   id                 STRING PRIMARY KEY,
   probability        DOUBLE DEFAULT 1.0,
   strength           DOUBLE DEFAULT 1.0,
   mechanism          STRING,
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";
pub const CREATE_EDGE_NODE_TABLE_BECAUSE_OF: &str = r"
   id                 STRING PRIMARY KEY,
   probability        DOUBLE DEFAULT 1.0,
   strength           DOUBLE DEFAULT 1.0,
   explanation        STRING,
   label              STRING,
   label_resolved     STRING,
   learned_at         TIMESTAMP,
   expire_at          TIMESTAMP,
   created_At         TIMESTAMP,
   updated_at         TIMESTAMP,
   layer              INT16 DEFAULT 0,
   context            STRING
";
pub const ALL_EDGE_NODE_DDLS: &[&str] = &[
    CREATE_EDGE_NODE_TABLE_CAUSES,
    CREATE_EDGE_NODE_TABLE_PREVENTS,
    CREATE_EDGE_NODE_TABLE_SIMILAR,
    CREATE_EDGE_NODE_TABLE_HAS_PROPERTY,
    CREATE_EDGE_NODE_TABLE_BECAUSE_OF,
    CREATE_EDGE_NODE_TABLE_CONTAINS,
    CREATE_EDGE_NODE_TABLE_LEADS_TO,
];
