// Time tree schema - seeds initial time hierarchy and abstract time nodes.
const TIMETREE_YEAR: &str = r"
MERGE (t:Time {id: 't-2026'}) SET t.granularity = 'year', t.starts_at = timestamp('2026-01-01'), t.ends_at = timestamp('2026-12-31'), t.label = '2026', t.label_resolved = '2026';
";
const TIMETREE_MONTH: &str = r"
MERGE (t:Time {id: 't-2026-11'}) SET t.granularity = 'month', t.starts_at = timestamp('2026-11-01'), t.ends_at = timestamp('2026-11-30'), t.label = 'November 2026', t.label_resolved = '2026-11';
";
const TIMETREE_DAY: &str = r"
MERGE (t:Time {id: 't-2026-11-03'}) SET t.granularity = 'day', t.starts_at = timestamp('2026-11-03'), t.ends_at = timestamp('2026-11-03'), t.label = 'November 3, 2026', t.label_resolved = '2026-11-03';
";
const TIMETREE_WEEK: &str = r"
MERGE (t:Time {id: 't-2026-w45'}) SET t.granularity = 'week', t.starts_at = timestamp('2026-11-02'), t.ends_at = timestamp('2026-11-08'), t.label = 'week 45, 2026', t.label_resolved = '2026-w45';
";
const WIRE_THE_HIERARCHY: &str = r"
MATCH (y:Time {id: 't-2026'}), (m:Time {id: 't-2026-11'}) MERGE (y)-[:TIME_HIERARCHY]->(m);
MATCH (m:Time {id: 't-2026-11'}), (d:Time {id: 't-2026-11-03'}) MERGE (m)-[:TIME_HIERARCHY]->(d);
MATCH (w:Time {id: 't-2026-w45'}), (d:Time {id: 't-2026-11-03'}) MERGE (w)-[:TIME_HIERARCHY]->(d);
";
// Abstract time nodes
const AT_FUTURE: &str = r"
MERGE (t:AbstractTime {id: 'at-future'}) SET t.semantics = 'unbounded forward', t.label = 'future', t.label_resolved = 'future (unbounded)';
";
const AT_NOW: &str = r"
MERGE (t:AbstractTime {id: 'at-now'}) SET t.semantics = 'current moment', t.label = 'now', t.label_resolved = 'now';
";
pub const ALL_TIME_TREE_DDLS: &[&str] = &[
    TIMETREE_YEAR,
    TIMETREE_MONTH,
    TIMETREE_DAY,
    TIMETREE_WEEK,
    WIRE_THE_HIERARCHY,
    AT_FUTURE,
    AT_NOW,
];
