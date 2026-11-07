use chrono::{DateTime, Utc};
use lbug::{Connection, Value};

use crate::db::DbResult;
use crate::nodes::Event;

use super::fetch::get_event;
use crate::mutations::chrono_to_time;

/// Returns all events whose `learned_at` falls within [start, end].
pub fn events_in_range(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> DbResult<Vec<Event>> {
    // Note: use start_ts/end_ts — `end` is a reserved Cypher keyword and
    // `$end` breaks the parser.
    let cypher =
        "MATCH (e:Event) WHERE e.learned_at >= $start_ts AND e.learned_at <= $end_ts RETURN e.id;";
    let mut stmt = conn.prepare(cypher)?;
    let result = conn.execute(
        &mut stmt,
        vec![
            ("start_ts", Value::Timestamp(chrono_to_time(&start))),
            ("end_ts", Value::Timestamp(chrono_to_time(&end))),
        ],
    )?;

    let mut events = Vec::new();
    for row in result {
        if let Value::String(id) = &row[0]
            && let Some(event) = get_event(conn, id)?
        {
            events.push(event);
        }
    }

    Ok(events)
}
