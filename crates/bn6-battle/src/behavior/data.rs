//! The content's data as scripts read it: the frozen global `data` (typed
//! for editors in content/bn6/core.d.luau). What is left of it is the hit
//! regions by number, which content model v2's last steps replace with
//! region definitions:
//!
//! ```text
//! data.regions[region]      a hit region's panels, [dx, dy] each (PanelOffsetListsPointerTable)
//! ```

use bn6_content_api::{Data, DataKey};
use serde::Serialize;

use crate::content::Content;

/// A serializable value as data (its pack-file form).
fn value<T: Serialize>(v: &T) -> Data {
    from_json(serde_json::to_value(v).expect("content data serializes"))
}

fn from_json(v: serde_json::Value) -> Data {
    use serde_json::Value as J;
    match v {
        J::Null => Data::Nil,
        J::Bool(b) => Data::Bool(b),
        J::Number(n) => Data::Int(n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)).expect("content numbers are integers")),
        J::String(s) => Data::Str(s),
        J::Array(items) => Data::List(items.into_iter().map(from_json).collect()),
        J::Object(fields) => Data::Map(fields.into_iter().map(|(k, v)| (DataKey::Str(k), from_json(v))).collect()),
    }
}

/// A table by id.
fn by_id<T>(items: impl IntoIterator<Item = (i64, T)>, f: impl Fn(&T) -> Data) -> Data {
    Data::Map(items.into_iter().map(|(id, v)| (DataKey::Int(id), f(&v))).collect())
}

/// The `data` global for `content`.
pub fn script_data(c: &Content) -> Data {
    Data::map([("regions", by_id(c.regions.iter().enumerate().map(|(i, r)| (i as i64, r)), |r| value(*r)))])
}
