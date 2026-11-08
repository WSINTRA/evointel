// Edge-node poly-relations: each edge type has a FROM_* and TO_* rel table.
// Syntax: multiple FROM ... TO ... pairs separated by commas (no pipes).

// Contains
pub const CONTAINS: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_Contains(
    FROM Memory TO Contains,
    FROM Event TO Contains,
    FROM Fact TO Contains,
    FROM Note TO Contains,
    FROM Time TO Contains,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_Contains(
    FROM Contains TO Memory,
    FROM Contains TO Event,
    FROM Contains TO Fact,
    FROM Contains TO Entity,
    FROM Contains TO Time,
    role STRING DEFAULT 'target'
);
";

// LeadsTo
pub const LEADS_TO: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_LeadsTo(
    FROM Memory TO LeadsTo,
    FROM Event TO LeadsTo,
    FROM Fact TO LeadsTo,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_LeadsTo(
    FROM LeadsTo TO Memory,
    FROM LeadsTo TO Event,
    FROM LeadsTo TO Fact,
    FROM LeadsTo TO AbstractTime,
    role STRING DEFAULT 'target'
);
";

// Prevents
pub const PREVENTS: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_Prevents(
    FROM Event TO Prevents,
    FROM Fact TO Prevents,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_Prevents(
    FROM Prevents TO Event,
    FROM Prevents TO Fact,
    FROM Prevents TO Memory,
    role STRING DEFAULT 'target'
);
";

// Causes
pub const CAUSES: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_Causes(
    FROM Memory TO Causes,
    FROM Event TO Causes,
    FROM Fact TO Causes,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_Causes(
    FROM Causes TO Event,
    FROM Causes TO Fact,
    role STRING DEFAULT 'target'
);
";

// BecauseOf
pub const BECAUSE_OF: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_BecauseOf(
    FROM Memory TO BecauseOf,
    FROM Event TO BecauseOf,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_BecauseOf(
    FROM BecauseOf TO Event,
    FROM BecauseOf TO Fact,
    role STRING DEFAULT 'target'
);
";

// Similar
pub const SIMILAR: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_Similar(
    FROM Note TO Similar,
    FROM Entity TO Similar,
    FROM Fact TO Similar,
    FROM Event TO Similar,
    FROM Memory TO Similar,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_Similar(
    FROM Similar TO Note,
    FROM Similar TO Entity,
    FROM Similar TO Fact,
    FROM Similar TO Event,
    FROM Similar TO Memory,
    role STRING DEFAULT 'target'
);
";

// HasProperty
pub const HAS_PROPERTY: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_HasProperty(
    FROM Note TO HasProperty,
    FROM Entity TO HasProperty,
    FROM Fact TO HasProperty,
    FROM Event TO HasProperty,
    FROM Memory TO HasProperty,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_HasProperty(
    FROM HasProperty TO Entity,
    role STRING DEFAULT 'target'
);
";

// Before
pub const BEFORE: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_Before(
    FROM Note TO Before,
    FROM Time TO Before,
    FROM Event TO Before,
    FROM AbstractTime TO Before,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_Before(
    FROM Before TO Time,
    FROM Before TO Event,
    FROM Before TO AbstractTime,
    role STRING DEFAULT 'target'
);
";

// After
pub const AFTER: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_After(
    FROM Note TO After,
    FROM Time TO After,
    FROM Event TO After,
    FROM AbstractTime TO After,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_After(
    FROM After TO Time,
    FROM After TO Event,
    FROM After TO AbstractTime,
    role STRING DEFAULT 'target'
);
";

// During
pub const DURING: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_During(
    FROM Note TO During,
    FROM Time TO During,
    FROM Event TO During,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_During(
    FROM During TO Time,
    FROM During TO Event,
    role STRING DEFAULT 'target'
);
";

// ValidFrom
pub const VALID_FROM: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_ValidFrom(
    FROM Time TO ValidFrom,
    FROM AbstractTime TO ValidFrom,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_ValidFrom(
    FROM ValidFrom TO Note,
    FROM ValidFrom TO Fact,
    FROM ValidFrom TO Event,
    FROM ValidFrom TO Memory,
    role STRING DEFAULT 'target'
);
";

// ValidTo
pub const VALID_TO: &str = "
CREATE REL TABLE IF NOT EXISTS FROM_ValidTo(
    FROM Time TO ValidTo,
    FROM AbstractTime TO ValidTo,
    role STRING DEFAULT 'source'
);
CREATE REL TABLE IF NOT EXISTS TO_ValidTo(
    FROM ValidTo TO Note,
    FROM ValidTo TO Fact,
    FROM ValidTo TO Event,
    FROM ValidTo TO Memory,
    role STRING DEFAULT 'target'
);
";

// Time Tree
const TIMETREE: &str = "
CREATE REL TABLE IF NOT EXISTS TIME_HIERARCHY(FROM Time TO Time, ONE_MANY);
";

pub const ALL_POLY_RELATIONS: &[&str] = &[
    CONTAINS,
    LEADS_TO,
    PREVENTS,
    CAUSES,
    BECAUSE_OF,
    SIMILAR,
    HAS_PROPERTY,
    BEFORE,
    AFTER,
    DURING,
    VALID_FROM,
    VALID_TO,
    TIMETREE,
];
