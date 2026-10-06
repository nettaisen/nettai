//! What a tool shows of a side's setup, pane by pane: the game's rules'
//! `panes` (docs/design/rules-in-luau.md, "c3b"), read. The rules declare
//! each pane's title, whether it is shown for a side, and how it shows its
//! facts, each by a view kind this module knows and no game's feature:
//!
//! - `flag`: a flag;
//! - `number`: a number (`format = "time"`: frames as mm:ss.cc);
//! - `pick`: one of a few (an enum's variants; a definition of the field's
//!   registry or an entry of its collection, `offered(side)` narrowing them;
//!   an entry of another list field, `of`; numbers by name, `choices`);
//! - `list`: rows, added, removed and reordered up to its room (`fixed`:
//!   the rows stay; `checklist`: a box for each one offered,
//!   `offered(side)`; `summary(side)`: a line above; a record's fields
//!   side by side, `columns` giving their views);
//! - `grid`: pieces placed on a board (`board(side)`: rows of `o`, a cell a
//!   piece may cover, `f`, a margin it may overhang but not stand wholly
//!   on, `.` none; each piece a record of the list, its fields named by
//!   `piece`, `x`, `y`, `rotation`, `color` and `toggles`; `shape(piece,
//!   toggles...)` its rows of `#` and `.` before turning, centered;
//!   `colors(piece)`, `palette`, `badge(piece)` how it looks).
//!
//! A function is named by its path among the rules' `panes`
//! (`panes.2.fields.1.board`), which `Battle::call_pane` calls on a round of
//! the match: the tool's to call. Whether a side's setup is right is the
//! rules' `validate`'s, not a view's.

use nettai_battle::content::Content;
use nettai_content_api::{Data, FieldType, Registry, RULESET_KEY};

/// A pane: its title (a key of the locales' `[setup]` table, else said as
/// it is), whether it is shown for a side (a function's path; none: it
/// is), and its facts' views.
#[derive(Clone, Debug, PartialEq)]
pub struct Pane {
    pub title: String,
    pub shown: Option<String>,
    pub fields: Vec<FieldPane>,
}

/// A fact's view in a pane: the setup field, its label (a key, else the
/// field's name), its view, and its path among the panes (its functions'
/// paths are under it).
#[derive(Clone, Debug, PartialEq)]
pub struct FieldPane {
    pub field: String,
    pub label: Option<String>,
    pub view: View,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum View {
    Flag,
    Number { time: bool },
    Pick(Pick),
    List(List),
    Grid(Grid),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pick {
    /// Numbers by name (a board's sizes); none: the field's own choices.
    pub choices: Vec<(i64, String)>,
    /// The function that narrows the definitions offered.
    pub offered: Option<String>,
    /// An entry of this other list field (its place in it, from 0).
    pub of: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct List {
    pub fixed: bool,
    pub checklist: bool,
    pub offered: Option<String>,
    pub summary: Option<String>,
    /// A record's fields' views, by field; a field not named, its type's.
    pub columns: Vec<(String, View)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub board: String,
    /// Rows to set apart (from 0), each with a key that says why.
    pub rows_of_note: Vec<(usize, String)>,
    pub piece: String,
    pub x: String,
    pub y: String,
    pub rotation: String,
    pub color: Option<String>,
    pub toggles: Vec<String>,
    pub shape: String,
    pub colors: Option<String>,
    /// Colors by name, 0xRRGGBB.
    pub palette: Vec<(String, u32)>,
    pub badge: Option<String>,
}

/// The game's rules' panes, read; none where they declare none. An error
/// names what isn't a pane.
pub fn panes(content: &Content) -> Result<Vec<Pane>, String> {
    let Some(rules) = content.defs.definitions.get(Registry::Rules, RULESET_KEY) else { return Ok(Vec::new()) };
    let at = format!("{}.luau: rules: panes", rules.module);
    let items = match rules.spec.field("panes") {
        Data::Nil => return Ok(Vec::new()),
        Data::List(items) => items,
        _ => return Err(format!("{at}: a list of panes")),
    };
    let setup = content.defs.rules().map(|r| content.defs.schema(r.setup));
    let function = |d: &Data, path: &str| -> Result<Option<String>, String> {
        match d {
            Data::Nil => Ok(None),
            Data::Function if content.defs.rules().and_then(|r| r.pane_function(path)).is_some() => Ok(Some(path.to_string())),
            _ => Err(format!("{at}: `{path}` is a function")),
        }
    };
    let string = |d: &Data, path: &str| -> Result<Option<String>, String> {
        match d {
            Data::Nil => Ok(None),
            Data::Str(s) => Ok(Some(s.clone())),
            _ => Err(format!("{at}: `{path}` is a name")),
        }
    };
    let mut out = Vec::new();
    for (i, p) in items.iter().enumerate() {
        let path = format!("panes.{}", i + 1);
        let title = string(p.field("title"), &format!("{path}.title"))?.ok_or_else(|| format!("{at}: `{path}` has no `title`"))?;
        let shown = function(p.field("shown"), &format!("{path}.shown"))?;
        let Data::List(fields) = p.field("fields") else { return Err(format!("{at}: `{path}.fields` is a list of views")) };
        let mut views = Vec::new();
        for (j, f) in fields.iter().enumerate() {
            let fpath = format!("{path}.fields.{}", j + 1);
            let field = string(f.field("field"), &format!("{fpath}.field"))?.ok_or_else(|| format!("{at}: `{fpath}` names no `field`"))?;
            let ty = setup
                .and_then(|s| s.index_of(&field).map(|k| s.field(k).ty.clone()))
                .ok_or_else(|| format!("{at}: `{fpath}`: the setup has no field `{field}`"))?;
            let label = string(f.field("label"), &format!("{fpath}.label"))?;
            let view = read_view(f, &ty, &fpath, &at, &function, &string)?;
            views.push(FieldPane { field, label, view, path: fpath });
        }
        out.push(Pane { title, shown, fields: views });
    }
    Ok(out)
}

type Reader<'a> = dyn Fn(&Data, &str) -> Result<Option<String>, String> + 'a;

/// The view `f` declares of a fact of type `ty`.
fn read_view(f: &Data, ty: &FieldType, path: &str, at: &str, function: &Reader, string: &Reader) -> Result<View, String> {
    let kind = string(f.field("view"), &format!("{path}.view"))?;
    let kind = kind.unwrap_or_else(|| default_kind(ty).to_string());
    let wrong = |what: &str| Err(format!("{at}: `{path}`: a {kind} view of {ty}: {what}"));
    Ok(match kind.as_str() {
        "flag" => match ty {
            FieldType::Bool => View::Flag,
            _ => return wrong("a flag is a bool's"),
        },
        "number" => match ty {
            t if crate::facts::range(t).is_some() || matches!(t, FieldType::OptionalU8) => View::Number { time: string(f.field("format"), &format!("{path}.format"))?.as_deref() == Some("time") },
            _ => return wrong("a number is an integer's"),
        },
        "pick" => {
            let mut choices = Vec::new();
            match f.field("choices") {
                Data::Nil => {}
                Data::Map(entries) => {
                    for (k, v) in entries {
                        let n: i64 = k.to_string().parse().map_err(|_| format!("{at}: `{path}.choices` is keyed by numbers"))?;
                        let name = v.str().ok_or_else(|| format!("{at}: `{path}.choices.{k}` is a name"))?;
                        choices.push((n, name.to_string()));
                    }
                }
                Data::List(names) => {
                    for (n, v) in names.iter().enumerate() {
                        choices.push((n as i64 + 1, v.str().ok_or_else(|| format!("{at}: `{path}.choices` holds names"))?.to_string()));
                    }
                }
                _ => return wrong("`choices` names numbers"),
            }
            let of = string(f.field("of"), &format!("{path}.of"))?;
            match ty {
                FieldType::Enum(_) | FieldType::Ref(..) => {}
                t if (crate::facts::range(t).is_some() || matches!(t, FieldType::OptionalU8)) && (!choices.is_empty() || of.is_some()) => {}
                _ => return wrong("a pick is an enum's, a definition's, or a number's with `choices` or `of`"),
            }
            View::Pick(Pick { choices, offered: function(f.field("offered"), &format!("{path}.offered"))?, of })
        }
        "list" => {
            let (FieldType::List(elem, _) | FieldType::Array(elem, _)) = ty else { return wrong("a list is a list's or an array's") };
            let flag = |name: &str| -> Result<bool, String> {
                match f.field(name) {
                    Data::Nil => Ok(false),
                    Data::Bool(b) => Ok(*b),
                    _ => Err(format!("{at}: `{path}.{name}` is a flag")),
                }
            };
            let checklist = flag("checklist")?;
            if checklist && !matches!(**elem, FieldType::Ref(..)) {
                return wrong("a checklist is a list of definitions'");
            }
            let mut columns = Vec::new();
            if let Data::Map(cols) = f.field("columns") {
                let FieldType::Record(fields) = &**elem else { return wrong("`columns` are a list of records'") };
                for (k, c) in cols {
                    let name = k.to_string();
                    let ty = fields.index_of(&name).map(|i| fields.field(i).ty.clone()).ok_or_else(|| format!("{at}: `{path}.columns`: its records have no field `{name}`"))?;
                    columns.push((name.clone(), read_view(c, &ty, &format!("{path}.columns.{name}"), at, function, string)?));
                }
            }
            View::List(List {
                fixed: flag("fixed")?,
                checklist,
                offered: function(f.field("offered"), &format!("{path}.offered"))?,
                summary: function(f.field("summary"), &format!("{path}.summary"))?,
                columns,
            })
        }
        "grid" => {
            let FieldType::List(elem, _) = ty else { return wrong("a grid is a list of records'") };
            let FieldType::Record(fields) = &**elem else { return wrong("a grid is a list of records'") };
            let name = |k: &str, need: bool| -> Result<Option<String>, String> {
                let v = string(f.field(k), &format!("{path}.{k}"))?;
                match (v, need) {
                    (Some(n), _) if fields.index_of(&n).is_none() => Err(format!("{at}: `{path}.{k}`: its records have no field `{n}`")),
                    (None, true) => Err(format!("{at}: `{path}` names no `{k}`")),
                    (v, _) => Ok(v),
                }
            };
            let need = |d: Option<String>, k: &str| d.ok_or_else(|| format!("{at}: `{path}` has no `{k}`"));
            let toggles = match f.field("toggles") {
                Data::Nil => Vec::new(),
                Data::List(l) => l.iter().map(|t| t.str().map(str::to_string).ok_or_else(|| format!("{at}: `{path}.toggles` are names"))).collect::<Result<_, _>>()?,
                _ => return wrong("`toggles` are names of its records' flags"),
            };
            for t in &toggles {
                if fields.index_of(t).is_none() {
                    return Err(format!("{at}: `{path}.toggles`: its records have no field `{t}`"));
                }
            }
            let rows_of_note = match f.field("rows_of_note") {
                Data::Nil => Vec::new(),
                Data::Map(entries) => entries
                    .iter()
                    .map(|(k, v)| {
                        let row: usize = k.to_string().parse().map_err(|_| format!("{at}: `{path}.rows_of_note` is keyed by rows"))?;
                        let why = v.str().ok_or_else(|| format!("{at}: `{path}.rows_of_note.{k}` is a name"))?;
                        Ok((row.checked_sub(1).ok_or_else(|| format!("{at}: `{path}.rows_of_note`: rows are from 1"))?, why.to_string()))
                    })
                    .collect::<Result<_, String>>()?,
                _ => return wrong("`rows_of_note` names rows"),
            };
            let palette = match f.field("palette") {
                Data::Nil => Vec::new(),
                Data::Map(entries) => entries
                    .iter()
                    .map(|(k, v)| v.int().map(|c| (k.to_string(), c as u32)).ok_or_else(|| format!("{at}: `{path}.palette.{k}` is a color, 0xRRGGBB")))
                    .collect::<Result<_, String>>()?,
                _ => return wrong("`palette` is colors by name"),
            };
            View::Grid(Grid {
                board: need(function(f.field("board"), &format!("{path}.board"))?, "board")?,
                rows_of_note,
                piece: name("piece", true)?.expect("needed"),
                x: name("x", true)?.expect("needed"),
                y: name("y", true)?.expect("needed"),
                rotation: name("rotation", true)?.expect("needed"),
                color: name("color", false)?,
                toggles,
                shape: need(function(f.field("shape"), &format!("{path}.shape"))?, "shape")?,
                colors: function(f.field("colors"), &format!("{path}.colors"))?,
                palette,
                badge: function(f.field("badge"), &format!("{path}.badge"))?,
            })
        }
        other => return Err(format!("{at}: `{path}`: no view {other:?} (flag, number, pick, list, grid)")),
    })
}

/// The view a fact of type `ty` has where none is declared.
pub fn default_kind(ty: &FieldType) -> &'static str {
    match ty {
        FieldType::Bool => "flag",
        FieldType::Enum(_) | FieldType::Ref(..) => "pick",
        FieldType::List(..) | FieldType::Array(..) => "list",
        _ => "number",
    }
}

/// The setup fields the panes show, by name.
pub fn shown_fields(panes: &[Pane]) -> Vec<&str> {
    panes.iter().flat_map(|p| p.fields.iter().map(|f| f.field.as_str())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both games' rules declare their panes, and each reads: its fields
    /// the setup's, its functions the rules'.
    #[test]
    fn the_games_panes_read() {
        for (content, game) in [(crate::testing::exe6_content(), "exe6"), (crate::testing::exe5_content(), "exe5")] {
            let panes = panes(&content).unwrap_or_else(|e| panic!("{game}: {e}"));
            assert!(!panes.is_empty(), "{game}");
            let fields = shown_fields(&panes);
            for f in ["patch_cards", "navicust_programs", "sp_times"] {
                assert!(fields.contains(&f), "{game}: {f} in no pane ({fields:?})");
            }
            let grid = panes.iter().flat_map(|p| &p.fields).find_map(|f| if let View::Grid(g) = &f.view { Some(g) } else { None });
            assert!(grid.is_some_and(|g| g.piece == "program" && g.toggles == ["compressed"]), "{game}: the NaviCust's grid");
        }
    }

    /// A pane's functions answer a tool on a round of the match: EXE6's
    /// NaviCust grid, its board by the side's expansions, a program's shape
    /// (compressed or not), colors and badge; its patch cards' summary.
    #[test]
    fn the_panes_functions_answer_on_a_round() {
        use nettai_content_api::Value;
        let content = crate::testing::exe6_content();
        let mut m = crate::pick::live(&content, "exe6", 3, None).unwrap();
        crate::testing::set_navicust(&content, &mut m.sides[0], &[], 2);
        let mut b = crate::check::start(&content, &m).unwrap();
        let panes = panes(&content).unwrap();
        let (grid, path) = panes
            .iter()
            .flat_map(|p| &p.fields)
            .find_map(|f| if let View::Grid(g) = &f.view { Some((g.clone(), f.path.clone())) } else { None })
            .unwrap();
        assert_eq!(grid.board, format!("{path}.board"));
        let board = b.call_pane(&grid.board, 0, &[Value::Int(0)]).unwrap().unwrap();
        let Data::List(rows) = board else { panic!("{board:?}") };
        assert_eq!(rows.iter().filter_map(|r| r.str()).collect::<Vec<_>>()[1], "fooooof");
        let program = crate::ids::entry(&content, "exe6", "navicust_programs", "bugstop").unwrap();
        let arg = Value::Def(Registry::Entry, program.0);
        let shape = |compressed: bool, b: &mut nettai_battle::Battle| b.call_pane(&grid.shape, 0, &[arg, Value::Bool(compressed)]).unwrap().unwrap();
        assert_ne!(shape(false, &mut b), shape(true, &mut b), "BugStop compresses");
        let colors = b.call_pane(grid.colors.as_ref().unwrap(), 0, &[arg]).unwrap().unwrap();
        assert!(matches!(colors, Data::List(ref c) if !c.is_empty()), "{colors:?}");
        let summary = panes.iter().flat_map(|p| &p.fields).find_map(|f| match &f.view {
            View::List(l) if f.field == "patch_cards" => l.summary.clone(),
            _ => None,
        });
        assert_eq!(b.call_pane(&summary.unwrap(), 0, &[Value::Int(0)]).unwrap().unwrap(), Data::Str("0 MB of 80".into()));
        assert!(b.call_pane("panes.9", 0, &[]).is_none(), "no function there");
    }
}
