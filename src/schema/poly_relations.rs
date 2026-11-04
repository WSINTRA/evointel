// Contains
pub const CONTAINS: &str = r"
CREATE REL TABLE FROM_Contains(FROM Memory | Event | Fact | Note | Time TO Contains, role STRING DEFAULT 'source');
CREATE REL TABLE TO_Contains(FROM Contains TO Memory | Event | Fact | Entity | Time, role STRING DEFAULT 'target');
";
// LeadsTo
pub const LEADS_TO: &str = r"
CREATE REL TABLE FROM_LeadsTo(FROM Memory | Event | Fact TO LeadsTo, role STRING DEFAULT 'source');
CREATE REL TABLE TO_LeadsTo(FROM LeadsTo TO Memory | Event | Fact | AbstractTime, role STRING DEFAULT 'target');
";
// Prevents
pub const PREVENTS: &str = r"
CREATE REL TABLE FROM_Prevents(FROM Event | Fact TO Prevents, role STRING DEFAULT 'source');
CREATE REL TABLE TO_Prevents(FROM Prevents TO Event | Fact | Memory, role STRING DEFAULT 'target');
";
// Causes
pub const CAUSES: &str = r"
CREATE REL TABLE FROM_Causes(FROM Memory | Event | Fact TO Causes, role STRING DEFAULT 'source');
CREATE REL TABLE TO_Causes(FROM Causes TO Event | Fact, role STRING DEFAULT 'target');
";
// BecauseOf
pub const BECAUSE_OF: &str = r"
CREATE REL TABLE FROM_BecauseOf(FROM Memory | Event TO BecauseOf, role STRING DEFAULT 'source');
CREATE REL TABLE TO_BecauseOf(FROM BecauseOf TO Event | Fact, role STRING DEFAULT 'target');
";
// Similar
pub const SIMILAR: &str = r"
CREATE REL TABLE FROM_Similar(FROM Note | Entity | Fact | Event | Memory TO Similar, role STRING DEFAULT 'source');
CREATE REL TABLE TO_Similar(FROM Similar TO Note | Entity | Fact | Event | Memory, role STRING DEFAULT 'target');
";
// HasProperty
pub const HAS_PROPERTY: &str = r"
CREATE REL TABLE FROM_HasProperty(FROM Note | Entity | Fact | Event | Memory TO HasProperty, role STRING DEFAULT 'source');
CREATE REL TABLE TO_HasProperty(FROM HasProperty TO Entity, role STRING DEFAULT 'target');
";
// Before
pub const BEFORE: &str = r"
CREATE REL TABLE FROM_Before(FROM Note | Time | Event | AbstractTime TO Before, role STRING DEFAULT 'source');
CREATE REL TABLE TO_Before(FROM Before TO Time | Event | AbstractTime, role STRING DEFAULT 'target');
";
// After
pub const AFTER: &str = r"
CREATE REL TABLE FROM_After(FROM Note | Time | Event | AbstractTime TO After, role STRING DEFAULT 'source');
CREATE REL TABLE TO_After(FROM After TO Time | Event | AbstractTime, role STRING DEFAULT 'target');
";
// During
pub const DURING: &str = r"
CREATE REL TABLE FROM_During(FROM Note | Time | Event TO During, role STRING DEFAULT 'source');
CREATE REL TABLE TO_During(FROM During TO Time | Event, role STRING DEFAULT 'target');
";
// ValidFrom
pub const VALID_FROM: &str = r"
CREATE REL TABLE FROM_ValidFrom(FROM Time | AbstractTime TO ValidFrom, role STRING DEFAULT 'source');
CREATE REL TABLE TO_ValidFrom(FROM ValidFrom TO Note | Fact | Event | Memory, role STRING DEFAULT 'target');
";
// ValidTo
pub const VALID_TO: &str = r"
CREATE REL TABLE FROM_ValidTo(FROM Time | AbstractTime TO ValidTo, role STRING DEFAULT 'source');
CREATE REL TABLE TO_ValidTo(FROM ValidTo TO Note | Fact | Event | Memory, role STRING DEFAULT 'target');
";
// Time Tree
const TIMETREE: &str = r"
CREATE REL TABLE TIME_HIERARCHY(FROM Time TO Time, ONE_MANY);";
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
