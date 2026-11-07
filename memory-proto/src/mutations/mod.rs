mod content;
mod notes;

pub use content::{insert_entity, insert_event, insert_fact, insert_memory};
pub use notes::insert_note;

use chrono::{DateTime, Utc};
use lbug::{LogicalType, Value};

use crate::schema::EMBEDDING_DIM;

/// Converts a chrono `DateTime<Utc>` to `time::OffsetDateTime` (what lbug expects).
pub(crate) fn chrono_to_time(dt: &DateTime<Utc>) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(dt.timestamp()).expect("valid unix timestamp")
}

/// Converts an optional `DateTime<Utc>` to a lbug Value (Timestamp or Null).
pub(crate) fn chrono_to_time_opt(dt: Option<&DateTime<Utc>>) -> Value {
    match dt {
        Some(ts) => Value::Timestamp(chrono_to_time(ts)),
        None => Value::Null(LogicalType::Timestamp),
    }
}

/// Converts an `Option<Vec<f32>>` embedding to the appropriate lbug Value.
pub(crate) fn embedding_to_value(embedding: Option<&Vec<f32>>) -> Value {
    match embedding {
        Some(vec) => Value::Array(
            LogicalType::Float,
            vec.iter().map(|f| Value::Float(*f)).collect(),
        ),
        None => Value::Null(LogicalType::Array {
            child_type: Box::new(LogicalType::Float),
            num_elements: EMBEDDING_DIM as u64,
        }),
    }
}

/// Converts a slice of strings to a lbug string list Value (matches DDL `STRING[]`).
pub(crate) fn string_list_to_value(items: &[String]) -> Value {
    Value::List(
        LogicalType::String,
        items.iter().map(|s| Value::String(s.clone())).collect(),
    )
}
