//! What a save or a recording states of a player as EXE4's rules' setup
//! facts (content/exe4/rules/init.luau): its NaviCust's programs.

use crate::Compat;
use crate::save::Part;
use nettai_battle::Content;
use nettai_battle::rules::Fact;
use nettai_content_api::{Registry, Value};

/// A NaviCust list (the save's, in its order) as the setup's
/// `navicust_programs`: each part's program and color (compat's
/// navicust.toml), its center on the engine's 7x7 grid (the save's column
/// and row, from 0 on the 5x5 board, one more), its quarter turns and its
/// compression.
pub fn navicust<'c>(content: &'c Content, compat: &Compat, parts: &[Option<Part>]) -> Result<Vec<Fact<'c>>, String> {
    let mut out = Vec::new();
    for p in parts.iter().flatten() {
        let Some((key, color)) = compat.navicust_part(p.id)? else { continue };
        let program = content.defs.entry_in("navicust_programs", key).ok_or_else(|| format!("the content has no NaviCust program {key}"))?;
        if p.column > 4 || p.row > 4 || p.rotation > 3 {
            return Err(format!("NaviCust part {:#04x} at column {}, row {}, turned {}: off EXE4's 5x5 board", p.id, p.column, p.row, p.rotation));
        }
        let colors = match content.defs.definitions.get(Registry::Entry, &content.defs.entry(program).key).map(|d| d.spec.field("colors")) {
            Some(nettai_content_api::Data::List(colors)) => colors.iter().filter_map(|c| c.str()).collect(),
            _ => Vec::new(),
        };
        let color = colors.get(color as usize).copied().ok_or_else(|| format!("NaviCust part {:#04x}: {key} has no color {color}", p.id))?;
        out.push(Fact::Record(vec![
            ("program", Fact::Value(Value::Def(Registry::Entry, program.0))),
            ("color", Fact::Name(color)),
            ("x", Fact::Value(Value::Int(p.column as i64 + 1))),
            ("y", Fact::Value(Value::Int(p.row as i64 + 1))),
            ("rotation", Fact::Value(Value::Int(p.rotation as i64))),
            ("compressed", Fact::Value(Value::Bool(p.compressed))),
        ]));
    }
    Ok(out)
}
