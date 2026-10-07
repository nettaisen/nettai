//! What the Builds screen and the creator show, as the window's models,
//! and the app's side of them: what each action and press does, and the
//! screens shown again after.

use crate::app::{App, game_names};
use crate::builds::layout::{self, Tab};
use crate::builds::screen::{BuildsState, Did, Editor, EntrySpec, FilterSpec, Kit, PickSpec, RowSpec, Shown, Then, checked};
use crate::builds::{auto, cards, grid, import, presets, store};
use crate::games::{Names, one_line};
use crate::{
    BuildCard, BuildTab, CheckTile, Detail, EntryRow, FactRow, FilterChip, GridCell, GridPart, LeftKind, NavAction, PickRow, RowKind,
    Screen, StatRow, TabKind, UiSound,
};
use nettai_battle::content::{ChipClass, ChipFlags, Content, PlayerFact};
use nettai_content_api::{ChipHandle, EntryHandle, FieldType, FormHandle, Registry};
use nettai_match::Side;
use nettai_match::facts::{self, Stated};
use slint::{Color, ModelRc, SharedString, VecModel};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The window's model of `items`.
pub fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items))
}

pub fn strings(items: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    model(items.into_iter().map(SharedString::from).collect())
}

fn rgb(c: u32) -> Color {
    Color::from_rgb_u8((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// A class's letter, as the games' folders show it.
fn class_letter(c: ChipClass) -> &'static str {
    match c {
        ChipClass::Standard => "S",
        ChipClass::Mega => "M",
        ChipClass::Giga => "G",
        ChipClass::ProgramAdvance => "PA",
        ChipClass::Special => "?",
    }
}

pub fn class_key(c: ChipClass) -> &'static str {
    match c {
        ChipClass::Standard => "standard",
        ChipClass::Mega => "mega",
        ChipClass::Giga => "giga",
        ChipClass::ProgramAdvance => "program_advance",
        ChipClass::Special => "special",
    }
}


/// A card's lines, and its bugs, each run together.
fn card_lines(names: &Names, h: EntryHandle) -> (String, String) {
    let Some(lines) = cards::lines(names, h) else { return Default::default() };
    let join = |bug: bool| lines.iter().filter(|l| l.bug == bug).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" · ");
    (join(false), join(true))
}

/// What the creator shows of the editor's tab, in the window's language.
struct View<'a> {
    e: &'a Editor,
    names: Names<'a>,
}

impl View<'_> {
    fn c(&self) -> &Content {
        self.e.kit.content()
    }

    fn rows(&self) -> Vec<FactRow> {
        let e = self.e;
        let c = self.c();
        let names = &self.names;
        e.rows
            .iter()
            .map(|spec| {
                let mut row = FactRow { kind: RowKind::Name, ..FactRow::default() };
                match spec {
                    RowSpec::Name => row.value = e.name.as_str().into(),
                    RowSpec::Navi => {
                        row.kind = RowKind::Navi;
                        if let Some(n) = e.side.stated_navi(c) {
                            row.value = names.navi(n).into();
                            row.picture = e.kit.pictures.navi(n);
                        }
                        row.problem = e.said(PlayerFact::Navi.name(), None).join(" · ").into();
                    }
                    RowSpec::Fact(f) => {
                        let value = e.side.facts.get(c, f);
                        row.key = f.as_str().into();
                        row.fallback = layout::title(f).into();
                        (row.kind, row.value, row.on) = match &value {
                            Some(Stated::Flag(on)) => (RowKind::Flag, SharedString::default(), *on),
                            Some(Stated::Number(n) | Stated::Optional(Some(n))) => (RowKind::Number, n.to_string().into(), false),
                            Some(Stated::Optional(None)) => (RowKind::Number, "—".into(), false),
                            Some(Stated::Variant(v)) => (RowKind::Choice, v.as_deref().map_or("—".to_string(), str::to_uppercase).into(), false),
                            Some(Stated::List(items)) => {
                                (RowKind::Choice, items.iter().map(|v| if *v == Stated::Flag(true) { "■" } else { "□" }).collect::<String>().into(), false)
                            }
                            Some(v) => (RowKind::Choice, facts::shown(c, v).into(), false),
                            None => (RowKind::Choice, SharedString::default(), false),
                        };
                        row.problem = e.said(f, None).join(" · ").into();
                    }
                    RowSpec::Preset(f) => {
                        // (Its preset by name, which the window words; the
                        // face the round starts the navi with.)
                        row.kind = RowKind::Preset;
                        row.key = f.as_str().into();
                        row.fallback = layout::title(f).into();
                        let preset = presets::of(&e.game, f).and_then(|p| presets::current(c, &e.side, f, p));
                        row.value = preset.map_or("", |p| p.choice.as_str()).into();
                        if let Ok(Some(face)) = e.round.as_ref().map(|r| r.face.as_ref()) {
                            row.picture = e.kit.pictures.shown(face);
                        }
                        let fields = std::iter::once(f.clone()).chain(presets::others(&e.game, f));
                        row.problem = fields.flat_map(|x| e.said(&x, None)).collect::<Vec<_>>().join(" · ").into();
                    }
                    RowSpec::FromSave => row.kind = RowKind::FromSave,
                    RowSpec::Duplicate => row.kind = RowKind::Duplicate,
                    RowSpec::Delete => row.kind = if e.delete_armed { RowKind::DeleteConfirm } else { RowKind::Delete },
                    RowSpec::Time(f, i) => {
                        row.kind = RowKind::Time;
                        row.fallback = e.time_def(f, *i).map(|(r, h)| names.def(r, h)).unwrap_or_default().into();
                        row.value = nettai_match::sp_times::format(e.time_of(f, *i).unwrap_or(0)).into();
                        row.problem = e.said(f, Some(*i)).join(" · ").into();
                    }
                }
                row
            })
            .collect()
    }

    fn entries(&self) -> Vec<EntryRow> {
        let e = self.e;
        let c = self.c();
        let names = &self.names;
        let folder = e.side.folder(c);
        let key = e.tab().key().to_string();
        let listed = e.side.facts.get(c, &key);
        let data = matches!(e.tab(), Tab::Places).then(|| auto::AutoBattle::of_side(c, &e.side));
        let total = match e.tab() {
            Tab::Entries { total, .. } => total.clone(),
            _ => None,
        };
        e.entries
            .iter()
            .map(|spec| {
                let mut row = EntryRow::default();
                match *spec {
                    EntrySpec::Folder(i) => {
                        row.number = format!("{:02}", i + 1).into();
                        match folder.chips[i] {
                            Some(chip) => {
                                row.icon = e.kit.pictures.icon(chip.id);
                                row.name = names.chip(chip.id).into();
                                row.tail = chip.code.letter().to_string().into();
                            }
                            None => row.empty = true,
                        }
                        let mut marks = Vec::new();
                        if folder.regular == Some(i as u8) {
                            marks.push("REG");
                        }
                        if folder.tags.is_some_and(|(a, b)| a == i as u8 || b == i as u8) {
                            marks.push("TAG");
                        }
                        row.marks = marks.join(" ").into();
                        row.problem = e.said(&key, Some(i)).join(" · ").into();
                    }
                    EntrySpec::Listed(i) => {
                        row.number = format!("{:02}", i + 1).into();
                        let h = listed.as_ref().map(|v| v.defs()).and_then(|d| d.get(i).copied());
                        if let Some(h) = h {
                            row.name = names.def(Registry::Entry, h).into();
                            let (lines, bugs) = card_lines(names, EntryHandle(h));
                            (row.lines, row.bugs) = (lines.into(), bugs.into());
                            if let Some(n) = total.as_ref().and_then(|t| cards::number(names, EntryHandle(h), t)) {
                                row.tail = format!("{n} MB").into();
                            }
                        }
                        row.problem = e.said(&key, Some(i)).join(" · ").into();
                    }
                    EntrySpec::Heading(k) => {
                        row.heading = true;
                        if let Some(l) = e.kit.auto.as_ref().and_then(|a| a.lists.get(k)) {
                            let places = if l.len == 1 { format!("{}", l.start + 1) } else { format!("{}–{}", l.start + 1, l.start + l.len) };
                            row.name = format!("{} · {places}", l.name.replace('_', " ").to_uppercase()).into();
                        }
                    }
                    EntrySpec::Place(i) => {
                        row.number = (i + 1).to_string().into();
                        let d = data.as_ref().expect("the places' tab");
                        let entry = d.places[i];
                        match entry {
                            auto::Entry::Chip(h) => {
                                row.icon = e.kit.pictures.icon(h);
                                row.name = names.chip(h).into();
                            }
                            auto::Entry::Pattern(n) => row.name = format!("▸ P{}", n + 1).into(),
                            auto::Entry::Zero => row.name = "0".into(),
                            auto::Entry::Empty => row.empty = true,
                        }
                        if let Some(list) = e.kit.auto.as_ref().and_then(|a| a.list_of(i))
                            && !auto::like_the_game(c, list, entry)
                        {
                            row.note = "≠".into();
                        }
                        row.problem = e.said("auto_battle_places", Some(i)).join(" · ").into();
                    }
                    EntrySpec::Other(i) => {
                        row.number = (i + 1).to_string().into();
                        if let Some(Stated::List(items)) = &listed {
                            row.name = items.get(i).map(|v| facts::shown(c, v)).unwrap_or_default().into();
                        }
                    }
                }
                row
            })
            .collect()
    }

    fn tiles(&self) -> Vec<CheckTile> {
        let e = self.e;
        let c = self.c();
        let Tab::Checklist(f) = e.tab() else { return Vec::new() };
        let held = e.side.facts.get(c, f).map(|v| v.defs()).unwrap_or_default();
        let registry = facts::field(c, f).and_then(|f| match layout::element(f.ty) {
            FieldType::Ref(r, _) => Some(*r),
            _ => None,
        });
        e.tiles
            .iter()
            .map(|&h| {
                let mut tile = CheckTile { on: held.contains(&h), ..CheckTile::default() };
                if let Some(r) = registry {
                    tile.name = self.names.def(r, h).into();
                }
                if registry == Some(Registry::Form) {
                    tile.face = e.kit.pictures.face(FormHandle(h));
                    let form = c.form(FormHandle(h));
                    let mut about: Vec<String> = form.version.iter().map(|v| v.to_uppercase()).collect();
                    about.extend(form.soul.as_ref().map(|s| format!("{:?}", s.family).to_uppercase()));
                    tile.about = about.join(" · ").into();
                }
                tile
            })
            .collect()
    }

    fn picks(&self) -> Vec<PickRow> {
        let e = self.e;
        let c = self.c();
        let names = &self.names;
        let folder = e.side.folder(c);
        let key = e.tab().key().to_string();
        let held_defs = e.side.facts.get(c, &key).map(|v| v.defs()).unwrap_or_default();
        let data = matches!(e.tab(), Tab::Places).then(|| auto::AutoBattle::of_side(c, &e.side));
        let parts = if e.tab() == &Tab::Grid { grid::placed(c, &e.side) } else { Vec::new() };
        let total = match e.tab() {
            Tab::Entries { total, .. } => total.clone(),
            _ => None,
        };
        e.picks
            .iter()
            .map(|p| {
                let mut row = PickRow::default();
                match *p {
                    PickSpec::Chip(h) => {
                        let d = c.chip(h);
                        row.icon = e.kit.pictures.icon(h);
                        row.name = names.chip(h).into();
                        row.detail = format!("{} · {} MB", class_letter(d.class), d.mb).into();
                        row.dark = d.flags.has(ChipFlags::DARK);
                        let n = match &data {
                            Some(data) => data.places.iter().filter(|x| **x == auto::Entry::Chip(h)).count(),
                            None => folder.chips().filter(|x| x.id == h).count(),
                        };
                        if n > 0 {
                            row.held = format!("×{n}").into();
                        }
                        if e.tab() == &Tab::Folder {
                            row.options = strings(d.codes.iter().map(|code| code.letter().to_string()));
                        }
                    }
                    PickSpec::Entry(h) => {
                        row.name = names.def(Registry::Entry, h).into();
                        let (lines, bugs) = card_lines(names, EntryHandle(h));
                        (row.lines, row.bugs) = (lines.into(), bugs.into());
                        if let Some(n) = total.as_ref().and_then(|t| cards::number(names, EntryHandle(h), t)) {
                            row.detail = format!("{n} MB").into();
                        }
                        row.taken = held_defs.contains(&h);
                        if row.taken {
                            row.held = "✓".into();
                        }
                    }
                    PickSpec::Piece(h) => {
                        row.name = names.entry(EntryHandle(h)).into();
                        row.lines = names.description(EntryHandle(h)).map(|d| one_line(&d)).unwrap_or_default().into();
                        if let Some(g) = &e.kit.grid {
                            if g.plus.contains(&h) {
                                row.detail = "+".into();
                            }
                            let colors = g.colors.get(&h).cloned().unwrap_or_default();
                            row.options = strings(colors.iter().map(|_| String::new()));
                            row.swatches = model(colors.iter().map(|n| rgb(grid::rgb(Some(n)))).collect());
                        }
                        let n = parts.iter().filter(|x| x.piece == h).count();
                        if n > 0 {
                            row.held = format!("×{n}").into();
                        }
                    }
                }
                row
            })
            .collect()
    }

    fn filters(&self) -> Vec<FilterChip> {
        let e = self.e;
        e.filters
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let on = i == e.filter && !matches!(f, FilterSpec::Search);
                match f {
                    FilterSpec::Search => FilterChip { key: "search".into(), fallback: e.search.as_str().into(), on: !e.search.is_empty() },
                    FilterSpec::All => FilterChip { key: "all".into(), fallback: SharedString::default(), on },
                    FilterSpec::Class(k) => FilterChip { key: class_key(*k).into(), fallback: class_key(*k).to_uppercase().into(), on },
                    FilterSpec::List => FilterChip { key: "list".into(), fallback: SharedString::default(), on },
                }
            })
            .collect()
    }

    /// The focused thing, large.
    fn detail(&self) -> Detail {
        let e = self.e;
        let c = self.c();
        let names = &self.names;
        let chip = |h: ChipHandle| {
            let d = c.chip(h);
            Detail {
                picture: e.kit.pictures.art(h),
                art: true,
                title: names.chip(h).into(),
                sub: format!("{} · {} MB · {}", class_letter(d.class), d.mb, d.damage).into(),
                ..Detail::default()
            }
        };
        let card = |h: u16| {
            let (lines, bugs) = card_lines(names, EntryHandle(h));
            let mb = cards::number(names, EntryHandle(h), "mb").map_or(String::new(), |n| format!("{n} MB"));
            Detail { title: names.def(Registry::Entry, h).into(), sub: mb.into(), lines: lines.into(), bugs: bugs.into(), ..Detail::default() }
        };
        let piece = |h: u16| Detail {
            title: names.entry(EntryHandle(h)).into(),
            text: names.description(EntryHandle(h)).map(|d| one_line(&d)).unwrap_or_default().into(),
            ..Detail::default()
        };
        let focused_pick = if e.pane == 1 { e.picks.get(e.pick.max(0) as usize).copied() } else { None };
        let mut out = match (e.tab(), focused_pick) {
            (_, Some(PickSpec::Chip(h))) => chip(h),
            (_, Some(PickSpec::Entry(h))) => card(h),
            (_, Some(PickSpec::Piece(h))) if e.held.is_none() => piece(h),
            (Tab::Folder, None) => match e.entries.get(e.cursor.max(0) as usize) {
                Some(EntrySpec::Folder(i)) => match e.side.folder(c).chips[*i] {
                    Some(fc) => {
                        let mut d = chip(fc.id);
                        d.title = format!("{} {}", d.title, fc.code.letter()).into();
                        d
                    }
                    None => Detail { title: format!("{:02} —", i + 1).into(), ..Detail::default() },
                },
                _ => Detail::default(),
            },
            (Tab::Entries { field, .. }, None) => match e.entries.get(e.cursor.max(0) as usize) {
                Some(EntrySpec::Listed(i)) => e.side.facts.get(c, field).and_then(|v| v.defs().get(*i).copied()).map_or_else(Detail::default, card),
                _ => Detail::default(),
            },
            (Tab::Grid, _) => match &e.held {
                Some(h) => piece(h.piece),
                None => {
                    let parts = grid::placed(c, &e.side);
                    let under = e.kit.grid.as_ref().and_then(|g| g.occupied(&parts).get(&e.cell).copied());
                    match under.and_then(|i| parts.get(i).map(|p| (i, p.piece))) {
                        Some((i, h)) => Detail { problem: e.said(grid::PIECES_FIELD, Some(i)).join(" · ").into(), ..piece(h) },
                        None => Detail::default(),
                    }
                }
            },
            (Tab::Places, None) => match e.selected_place().map(|i| auto::AutoBattle::of_side(c, &e.side).places[i]) {
                Some(auto::Entry::Chip(h)) => chip(h),
                _ => Detail::default(),
            },
            _ => Detail::default(),
        };
        // (What the rules say of the focused entry.)
        if e.pane == 0 && out.problem.is_empty() {
            let entry = match e.entries.get(e.cursor.max(0) as usize) {
                Some(EntrySpec::Folder(i) | EntrySpec::Listed(i) | EntrySpec::Place(i)) => Some(*i),
                _ => None,
            };
            if let Some(i) = entry {
                out.problem = e.said(e.tab().key(), Some(i)).join(" · ").into();
            }
        }
        out
    }

    /// What the round starts the navi with.
    fn stats(&self) -> Vec<StatRow> {
        let Some(st) = self.e.stats() else { return Vec::new() };
        let row = |key: &str, value: String| StatRow { key: key.into(), value: value.into() };
        let mut out = vec![
            row("hp", st.max_hp.to_string()),
            row("attack", (st.attack + 1).to_string()),
            row("rapid", (st.rapid + 1).to_string()),
            row("charge", (st.charge + 1).to_string()),
            row("custom", st.custom_level.to_string()),
            row("mega", st.mega_level.to_string()),
            row("giga", st.giga_level.to_string()),
            row("regular", format!("{} MB", st.reg_up)),
        ];
        let mut abilities: Vec<&str> = [(st.super_armor, "SuperArmor"), (st.float_shoes, "FloatShoes"), (st.air_shoes, "AirShoes"), (st.undershirt, "UnderShirt")]
            .iter()
            .filter(|x| x.0)
            .map(|x| x.1)
            .collect();
        if let Some(s) = st.support {
            abilities.extend([(s.rush, "Rush"), (s.beat, "Beat"), (s.tango, "Tango")].iter().filter(|x| x.0).map(|x| x.1));
        }
        if !abilities.is_empty() {
            out.push(row("abilities", abilities.join(", ")));
        }
        let b = &st.bugs;
        for (on, key) in [
            (b.processing != 0, "processing"),
            (b.panel_trail_level != 0, "panel_trail"),
            (b.buster_blanks != 0, "buster_blanks"),
            (b.hit_status != 0, "hit_status"),
            (b.hp_drain != 0, "hp_drain"),
            (b.custom_drain != 0, "custom_drain"),
            (b.battle_start != 0, "battle_start"),
            (b.emotion != 0, "emotion"),
            (b.hand_shrink_turn != 0, "hand_shrink"),
            (b.custom_damage != 0, "custom_damage"),
            (st.support.is_none(), "support"),
        ] {
            if on {
                out.push(row("bug", key.to_string()));
            }
        }
        out
    }

    /// The tab's counts: the folder's chips and limits, a list's room.
    fn counts(&self) -> Vec<StatRow> {
        let e = self.e;
        let c = self.c();
        let row = |key: &str, value: String| StatRow { key: key.into(), value: value.into() };
        match e.tab() {
            Tab::Folder => {
                let f = e.side.folder(c);
                let count = |class: ChipClass| f.chips().filter(|x| c.chip(x.id).class == class).count();
                let at = |i: u8| f.chips.get(i as usize).copied().flatten();
                let mut out = vec![row("chips", format!("{}/{}", f.chips().count(), f.chips.len()))];
                if let Some(st) = e.stats() {
                    out.push(row("mega", format!("{}/{}", count(ChipClass::Mega), st.mega_level)));
                    out.push(row("giga", format!("{}/{}", count(ChipClass::Giga), st.giga_level)));
                    if c.defs.fact_field(PlayerFact::RegularChip).is_some() {
                        let mb = f.regular.and_then(at).map_or(0, |x| c.chip(x.id).mb);
                        out.push(row("regular", format!("{mb}/{} MB", st.reg_up)));
                    }
                }
                if let Some((a, b)) = f.tags {
                    let mb: u32 = [a, b].into_iter().filter_map(at).map(|x| c.chip(x.id).mb as u32).sum();
                    out.push(row("tags", format!("{mb} MB")));
                }
                out
            }
            Tab::Entries { field, total } => {
                let held = e.side.facts.get(c, field).map(|v| v.defs()).unwrap_or_default();
                let room = facts::field(c, field).map_or(0, |f| layout::room(f.ty));
                let mut out = vec![row("held", format!("{}/{room}", held.len()))];
                if let Some(t) = total {
                    let sum: i64 = held.iter().filter_map(|&h| cards::number(&self.names, EntryHandle(h), t)).sum();
                    out.push(row("mb", format!("{sum} MB")));
                }
                out
            }
            Tab::Checklist(f) => {
                let held = e.side.facts.get(c, f).map(|v| v.defs()).unwrap_or_default();
                let room = facts::field(c, f).map_or(0, |f| layout::room(f.ty));
                vec![row("held", format!("{}/{room}", held.len()))]
            }
            Tab::Places => vec![row("places", format!("{}/{}", auto::AutoBattle::of_side(c, &e.side).entries(), auto::PLACES))],
            _ => Vec::new(),
        }
    }

    /// The tab's problems tied to no entry (the NAVI tab's are the
    /// summary's).
    fn notes(&self) -> Vec<String> {
        let e = self.e;
        if e.tab() == &Tab::Navi {
            return Vec::new();
        }
        e.tab_problems(e.tab()).into_iter().filter(|p| p.entry.is_none()).map(|p| p.text.clone()).collect()
    }
}

/// The board as the window draws it.
struct BoardShown {
    n: i32,
    size: String,
    cells: Vec<GridCell>,
    parts: Vec<GridPart>,
    ghost: Vec<GridCell>,
    fits: bool,
    color: Color,
    hovered: i32,
}

fn board_shown(e: &Editor) -> BoardShown {
    let content = e.content();
    let c = &*content;
    let mut out = BoardShown { n: 7, size: String::new(), cells: Vec::new(), parts: Vec::new(), ghost: Vec::new(), fits: false, color: Color::default(), hovered: -1 };
    let (Some(g), Some(board)) = (&e.kit.grid, e.board()) else { return out };
    out.n = board.len() as i32;
    out.size = grid::size(board);
    for (y, row) in board.iter().enumerate() {
        for (x, &k) in row.iter().enumerate() {
            let kind = match k {
                b'o' => 1,
                b'f' => 2,
                _ => continue,
            };
            out.cells.push(GridCell { x: x as i32, y: y as i32, kind, line: g.command_line == Some(y) && kind == 1 });
        }
    }
    let parts = grid::placed(c, &e.side);
    for (i, p) in parts.iter().enumerate() {
        let bad = !e.said(grid::PIECES_FIELD, Some(i)).is_empty();
        for (x, y) in grid::cells(&g.shape(p.piece, p.compressed, p.rotation), p.x, p.y) {
            out.parts.push(GridPart { x, y, color: rgb(grid::rgb(p.color.as_deref())), plus: g.plus.contains(&p.piece), piece: i as i32, bad });
        }
    }
    match &e.held {
        Some(h) => {
            let shape = g.shape(h.piece, h.compressed, h.rotation);
            let (x, y) = (e.cell.0 - h.grab.0, e.cell.1 - h.grab.1);
            out.fits = g.fits(board, &parts, &shape, x, y);
            out.ghost = grid::cells(&shape, x, y)
                .into_iter()
                .filter(|&(cx, cy)| (0..out.n).contains(&cx) && (0..out.n).contains(&cy))
                .map(|(x, y)| GridCell { x, y, kind: 0, line: false })
                .collect();
            let colors = g.colors.get(&h.piece).cloned().unwrap_or_default();
            out.color = rgb(grid::rgb(colors.get(h.color as usize).map(String::as_str)));
        }
        None => out.hovered = g.occupied(&parts).get(&e.cell).map_or(-1, |&i| i as i32),
    }
    out
}

// ---- The app's side ---------------------------------------------------------------------------

impl App {
    /// The creator's kit of `game`, made the first time.
    fn kit(&mut self, game: &str) -> Option<Rc<Kit>> {
        if let Some(k) = self.builds.kits.get(game) {
            return Some(k.clone());
        }
        let ready = self.games.ready(game)?;
        let kit = Rc::new(Kit::new(game, ready));
        self.builds.kits.insert(game.to_string(), kit.clone());
        Some(kit)
    }

    /// The game the Builds screen shows.
    fn builds_game_id(&self) -> Option<String> {
        self.ready_games().get(self.builds.game).cloned()
    }

    /// The Builds screen's game's place among all the games (Play's).
    pub fn builds_game_index(&self) -> usize {
        let id = self.builds_game_id();
        self.games.list.iter().position(|e| Some(&e.id) == id.as_ref()).unwrap_or(0)
    }

    /// The Builds screen, as the builds on disk are.
    pub fn show_builds(&mut self) {
        let ui = self.ui();
        let ready = self.ready_games();
        self.builds.game = self.builds.game.min(ready.len().saturating_sub(1));
        ui.set_builds_games(strings(ready.iter().map(|g| game_names(g).0)));
        ui.set_builds_game_index(self.builds.game as i32);
        let game = self.builds_game_id();
        self.builds.shown.clear();
        let kit = game.as_deref().and_then(|g| self.kit(g));
        if let (Some(game), Some(kit)) = (&game, &kit) {
            let content = kit.content().clone();
            for listed in store::list(game) {
                let side = store::read(&content, game, &listed.text).map(|(_, mut s)| {
                    layout::as_built(&content, game, &mut s);
                    s
                });
                let problems = side.as_ref().map(|s| checked(&content, game, s).0).unwrap_or_default();
                self.builds.shown.push(Shown { listed, side, problems });
            }
        }
        let cards: Vec<BuildCard> = match &kit {
            Some(kit) => {
                let graphics = kit.ready.graphics(self.lang);
                let names = Names::of(kit.content(), &graphics);
                let c = kit.content();
                self.builds
                    .shown
                    .iter()
                    .map(|s| match &s.side {
                        Ok(side) => {
                            let navi = side.navi(c);
                            let folder = side.folder(c);
                            BuildCard {
                                name: s.listed.name.as_str().into(),
                                navi: names.navi(navi).into(),
                                face: kit.pictures.navi(navi),
                                chips: folder.chips().count() as i32,
                                total: folder.chips.len() as i32,
                                problems: s.problems.len() as i32,
                                note: s.problems.first().map(|p| p.text.clone()).unwrap_or_default().into(),
                            }
                        }
                        Err(why) => BuildCard { name: s.listed.name.as_str().into(), problems: -1, note: why.as_str().into(), ..BuildCard::default() },
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        let count = cards.len();
        ui.set_builds_cards(model(cards));
        self.builds.cursor = self.builds.cursor.min(count + 1);
        ui.set_builds_cursor(self.builds.cursor as i32);
        ui.set_builds_status(self.builds.status.as_str().into());
        ui.set_builds_folder(game.as_deref().map(|g| store::folder(g).display().to_string()).unwrap_or_default().into());
        ui.set_builds_imports(game.as_deref().is_some_and(import::reads_saves));
    }

    pub fn builds_nav(&mut self, a: NavAction) {
        let n = self.builds.shown.len() + 2;
        let games = self.ready_games().len();
        match a {
            NavAction::Up | NavAction::Down => {
                let to = if a == NavAction::Up { self.builds.cursor.checked_sub(1) } else { (self.builds.cursor + 1 < n).then_some(self.builds.cursor + 1) };
                if let Some(to) = to {
                    self.builds.cursor = to;
                    self.sound.play(UiSound::Cursor);
                }
            }
            NavAction::Previous | NavAction::Next | NavAction::Left | NavAction::Right if games > 1 => {
                let back = matches!(a, NavAction::Previous | NavAction::Left);
                self.builds.game = (self.builds.game + if back { games - 1 } else { 1 }) % games;
                self.builds.cursor = 0;
                self.builds.status.clear();
                self.sound.play(UiSound::Cursor);
            }
            NavAction::Confirm => return self.builds_activate(self.builds.cursor),
            NavAction::Back => {
                self.sound.play(UiSound::Back);
                self.go(Screen::Title);
                return;
            }
            _ => return,
        }
        self.show_builds();
    }

    pub fn builds_game(&mut self, i: usize) {
        if i != self.builds.game {
            self.builds.game = i;
            self.builds.cursor = 0;
            self.builds.status.clear();
            self.sound.play(UiSound::Cursor);
            self.show_builds();
        }
    }

    pub fn builds_point(&mut self, i: usize) {
        if i != self.builds.cursor {
            self.builds.cursor = i;
            self.sound.play(UiSound::Cursor);
            self.ui().set_builds_cursor(i as i32);
        }
    }

    pub fn builds_activate(&mut self, i: usize) {
        self.builds.cursor = i;
        let Some(game) = self.builds_game_id() else { return self.sound.play(UiSound::Refused) };
        let Some(kit) = self.kit(&game) else { return self.sound.play(UiSound::Refused) };
        let taken: Vec<String> = self.builds.shown.iter().map(|s| s.listed.name.clone()).collect();
        match i {
            0 => {
                let Ok(side) = Side::fresh(kit.content(), &game) else { return self.sound.play(UiSound::Refused) };
                let name = store::new_name("Build", &taken);
                let path = store::new_path(&game, &name);
                self.sound.play(UiSound::Pick);
                self.open_build(kit, &game, path, name, side, true);
            }
            1 => self.import_save(None),
            _ => {
                let Some(s) = self.builds.shown.get(i - 2) else { return };
                match &s.side {
                    Ok(side) => {
                        let (path, name, side) = (s.listed.path.clone(), s.listed.name.clone(), side.clone());
                        self.sound.play(UiSound::Pick);
                        self.open_build(kit, &game, path, name, side, false);
                    }
                    Err(why) => {
                        self.builds.status = why.clone();
                        self.sound.play(UiSound::Refused);
                        self.show_builds();
                    }
                }
            }
        }
    }

    /// Open a build in the creator (`new`: write it first).
    fn open_build(&mut self, kit: Rc<Kit>, game: &str, path: PathBuf, name: String, side: Side, new: bool) {
        let mut e = Editor::new(kit, game, path, name, side);
        if new {
            e.save();
        }
        self.builds.editor = Some(e);
        self.go(Screen::Build);
    }

    /// A save made into a new build of the Builds screen's game (`path`:
    /// a file dropped on the window; none: the system's file dialog asks).
    pub fn import_save(&mut self, path: Option<PathBuf>) {
        let Some(game) = self.builds_game_id() else { return self.sound.play(UiSound::Refused) };
        if !import::reads_saves(&game) {
            return self.sound.play(UiSound::Refused);
        }
        let Some(path) = path.or_else(pick_save) else { return };
        let Some(kit) = self.kit(&game) else { return };
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| import::side_of_save(kit.content(), &game, &bytes));
        match read {
            Ok((side, notes)) => {
                let taken: Vec<String> = self.builds.shown.iter().map(|s| s.listed.name.clone()).collect();
                let stem = path.file_stem().map_or("Save".to_string(), |s| s.to_string_lossy().into_owned());
                let name = store::new_name(&stem, &taken);
                let bpath = store::new_path(&game, &name);
                self.sound.play(UiSound::Pick);
                self.open_build(kit, &game, bpath, name, side, true);
                if let Some(e) = &mut self.builds.editor {
                    e.status = notes.join("; ");
                }
            }
            Err(why) => {
                self.builds.status = format!("{}: {why}", path.display());
                self.sound.play(UiSound::Refused);
                self.show_builds();
            }
        }
    }

    /// A file dropped on the window: a save, made into a build.
    pub fn dropped(&mut self, path: &Path) {
        match self.ui().get_screen() {
            Screen::Builds => self.import_save(Some(path.to_path_buf())),
            Screen::Build => self.build_from_save(Some(path.to_path_buf()), false),
            _ => {}
        }
    }

    /// The open build's side (or its auto battle data alone) from a save.
    fn build_from_save(&mut self, path: Option<PathBuf>, auto_battle: bool) {
        let Some(e) = &self.builds.editor else { return };
        if !import::reads_saves(&e.game) {
            return self.sound.play(UiSound::Refused);
        }
        let Some(path) = path.or_else(pick_save) else { return };
        let Some(e) = &mut self.builds.editor else { return };
        let content = e.content();
        let read = std::fs::read(&path).map_err(|x| x.to_string());
        let done = read.and_then(|bytes| {
            if auto_battle {
                import::auto_battle_of_save(&content, &e.game, &bytes, &mut e.side)
            } else {
                import::side_of_save(&content, &e.game, &bytes).map(|(side, notes)| {
                    e.side = side;
                    notes
                })
            }
        });
        let did = match done {
            Ok(notes) => {
                e.status = if notes.is_empty() { path.display().to_string() } else { notes.join("; ") };
                Did { sound: Some(UiSound::Pick), edited: true, lists: true, ..Did::default() }
            }
            Err(why) => {
                e.status = why;
                Did::refused()
            }
        };
        self.after(did);
    }

    /// Run `f` on the open build, then do what it did.
    fn on_build(&mut self, f: impl FnOnce(&mut Editor) -> Did) {
        let columns = self.ui().get_build_tile_columns().max(1) as usize;
        let Some(e) = self.builds.editor.as_mut() else { return };
        e.columns = columns;
        let did = f(e);
        self.after(did);
    }

    fn after(&mut self, did: Did) {
        if let Some(s) = did.sound {
            self.sound.play(s);
        }
        let Some(e) = self.builds.editor.as_mut() else { return };
        let mut lists = did.lists;
        if did.edited {
            e.save();
            e.check();
            e.work_out();
            lists = true;
        }
        match did.then {
            Then::Nothing => {}
            Then::Leave => {
                self.go(Screen::Builds);
                return;
            }
            Then::FromSave => return self.build_from_save(None, false),
            Then::AutoFromSave => return self.build_from_save(None, true),
            Then::Duplicate => {
                let (kit, game, name, side) = (e.kit.clone(), e.game.clone(), e.name.clone(), e.side.clone());
                let taken: Vec<String> = store::list(&game).into_iter().map(|l| l.name).collect();
                let name = store::new_name(&name, &taken);
                let path = store::new_path(&game, &name);
                let mut copy = Editor::new(kit, &game, path, name, side);
                copy.strings = e.strings.clone();
                copy.save();
                copy.status = format!("{} ✓", copy.name);
                self.builds.editor = Some(copy);
                lists = true;
            }
            Then::Delete => {
                let path = e.path.clone();
                if let Err(why) = std::fs::remove_file(&path) {
                    e.status = format!("{}: {why}", path.display());
                } else {
                    self.builds.editor = None;
                    self.go(Screen::Builds);
                    return;
                }
            }
        }
        self.show_build(lists);
    }

    /// The creator as the open build stands (`lists`: its lists too, not
    /// only where the keys are).
    pub fn show_build(&mut self, lists: bool) {
        let ui = self.ui();
        let Some(e) = &self.builds.editor else { return };
        let graphics = e.kit.ready.graphics(self.lang);
        let view = View { e, names: Names::of(e.kit.content(), &graphics) };
        if lists {
            ui.set_build_name(e.name.as_str().into());
            ui.set_build_game(game_names(&e.game).0.into());
            ui.set_build_problem_count(e.problems.len() as i32);
            let tabs: Vec<BuildTab> = e
                .layout
                .tabs
                .iter()
                .map(|t| {
                    let kind = match t {
                        Tab::Navi => TabKind::Navi,
                        Tab::Folder => TabKind::Folder,
                        Tab::Checklist(_) => TabKind::Checklist,
                        Tab::Grid => TabKind::Grid,
                        Tab::Entries { .. } => TabKind::Entries,
                        Tab::Times(_) => TabKind::Times,
                        Tab::Places => TabKind::Places,
                        Tab::Other(_) => TabKind::Other,
                    };
                    BuildTab { kind, key: t.key().into(), fallback: layout::title(t.key()).into(), problems: e.tab_problems(t).len() as i32 }
                })
                .collect();
            ui.set_build_tabs(model(tabs));
            ui.set_build_tab_index(e.tab as i32);
            ui.set_build_left(e.left_kind());
            ui.set_build_right(e.right_kind());
            ui.set_build_rows(model(view.rows()));
            ui.set_build_entries(model(view.entries()));
            ui.set_build_tiles(model(view.tiles()));
            ui.set_build_picks(model(view.picks()));
            ui.set_build_actions(model(e.actions.clone()));
            ui.set_build_filters(model(view.filters()));
            ui.set_build_notes(strings(view.notes()));
            ui.set_build_counts(model(view.counts()));
            if let Some(n) = e.side.stated_navi(e.kit.content()) {
                ui.set_build_face(e.kit.pictures.navi(n));
                ui.set_build_navi(view.names.navi(n).into());
            }
            ui.set_build_stats(model(view.stats()));
            ui.set_build_stopped(e.round.as_ref().err().cloned().unwrap_or_default().into());
            ui.set_build_problems(strings(e.problems.iter().map(|p| p.text.clone())));
        }
        // Where the keys are.
        ui.set_build_pane(e.pane as i32);
        ui.set_build_cursor(e.cursor);
        ui.set_build_pick(e.pick);
        ui.set_build_option(e.option as i32);
        ui.set_build_strip_buttons(model(e.strip()));
        ui.set_build_detail(view.detail());
        if e.tab() == &Tab::Grid {
            let b = board_shown(e);
            ui.set_build_board_n(b.n);
            ui.set_build_board_size(b.size.into());
            ui.set_build_cells(model(b.cells));
            ui.set_build_parts(model(b.parts));
            ui.set_build_ghost(model(b.ghost));
            ui.set_build_ghost_fits(b.fits);
            ui.set_build_ghost_color(b.color);
            ui.set_build_hovered(b.hovered);
            ui.set_build_cell_x(e.cell.0);
            ui.set_build_cell_y(e.cell.1);
        }
        ui.set_build_holding(e.held.is_some());
        ui.set_build_compresses(e.held.as_ref().zip(e.kit.grid.as_ref()).is_some_and(|(h, g)| g.compresses(h.piece)));
        ui.set_build_status(e.status.as_str().into());
        // What is being typed.
        let editing = e.editing.is_some();
        if editing != ui.get_build_editing() {
            let (label, text, hint) = e.edit_label();
            ui.set_build_edit_label(label.into());
            ui.set_build_edit_text(text.into());
            ui.set_build_edit_hint(hint.into());
            ui.set_build_editing(editing);
            if editing {
                ui.invoke_build_edit_focus();
            } else {
                ui.invoke_build_edit_leave();
                ui.invoke_focus_keys();
            }
        }
    }

    /// The creator, entered: the open build shown in the window's language.
    pub fn enter_build(&mut self) {
        let strings = self.builds.editor.as_ref().and_then(|e| e.kit.ready.graphics(self.lang).strings);
        if let Some(e) = &mut self.builds.editor {
            e.strings = strings;
            e.work_out();
        }
        self.show_build(true);
    }

    pub fn build_nav(&mut self, a: NavAction) {
        self.on_build(|e| e.nav(a));
    }

    pub fn build_tab(&mut self, i: usize) {
        self.on_build(|e| if i == e.tab { Did::default() } else { e.go_tab(i) });
    }

    /// A row, an entry, a tile or a pick clicked (`option`: one of its
    /// options, -1 the row itself).
    pub fn build_act(&mut self, pane: i32, i: i32, option: i32) {
        self.on_build(|e| {
            e.delete_armed &= pane == 0 && i == e.cursor;
            if e.editing.is_some() {
                return Did::default();
            }
            if pane == 1 {
                let again = e.pane == 1 && e.pick == i && option < 0;
                e.pane = 1;
                e.pick = i;
                e.strip = false;
                if option >= 0 {
                    e.option = option as usize;
                    return e.put(i as usize, option as usize);
                }
                if again && e.pick_options(i as usize) <= 1 {
                    return e.put(i as usize, 0);
                }
                return Did::cursor();
            }
            let again = e.pane == 0 && e.cursor == i;
            e.pane = 0;
            if !again {
                e.cursor = i;
                e.strip = false;
                e.option = option.max(0) as usize;
                if e.left_kind() == LeftKind::Tiles {
                    return e.toggle_tile(i as usize);
                }
                if e.tab() == &Tab::Places {
                    e.work_out_picks();
                    return Did::lists(UiSound::Cursor);
                }
                if option >= 0 {
                    return e.nav(NavAction::Confirm);
                }
                return Did::cursor();
            }
            if option >= 0 {
                e.option = option as usize;
            }
            e.nav(NavAction::Confirm)
        });
    }

    /// A row's value stepped with its arrows (-1: the board's size).
    pub fn build_step(&mut self, i: i32, by: i32) {
        self.on_build(|e| {
            e.pane = 0;
            e.cursor = i;
            if i < 0 {
                return e.step_board(by);
            }
            match e.rows.get(i as usize).cloned() {
                Some(row) => e.step_row(&row, by),
                None => Did::default(),
            }
        });
    }

    pub fn build_action(&mut self, k: usize) {
        self.on_build(|e| {
            e.pane = 0;
            e.cursor = -1;
            e.option = k;
            e.do_action(k)
        });
    }

    pub fn build_strip(&mut self, k: usize) {
        self.on_build(|e| e.do_strip(k));
    }

    pub fn build_filter(&mut self, k: usize) {
        self.on_build(|e| {
            e.pane = 1;
            e.pick = -1;
            e.option = k;
            e.do_filter(k)
        });
    }

    pub fn build_grid_hover(&mut self, x: i32, y: i32) {
        let Some(e) = self.builds.editor.as_mut() else { return };
        let n = e.board_n();
        if !(0..n).contains(&x) || !(0..n).contains(&y) || (e.cell == (x, y) && e.pane == 0 && e.cursor >= 0) {
            return;
        }
        e.cell = (x, y);
        e.pane = 0;
        e.cursor = 0;
        self.show_build(false);
    }

    pub fn build_grid_press(&mut self, x: i32, y: i32, right: bool) {
        self.on_build(|e| {
            let n = e.board_n();
            if !(0..n).contains(&x) || !(0..n).contains(&y) {
                return Did::default();
            }
            e.cell = (x, y);
            e.pane = 0;
            e.cursor = 0;
            if !right {
                return e.press_cell(x, y);
            }
            let Some(g) = e.kit.grid.clone() else { return Did::default() };
            let content = e.content();
            // (The piece in hand taken off; one placed, turned where it is.)
            if e.held.is_some() {
                return e.nav(NavAction::Remove);
            }
            let parts = grid::placed(&content, &e.side);
            match g.occupied(&parts).get(&(x, y)) {
                Some(&i) => Did::edit(grid::edit(&g, &content, &mut e.side, &mut e.held, grid::Edit::TurnAt(i))),
                None => Did::default(),
            }
        });
    }

    pub fn build_grid_wheel(&mut self, up: bool) {
        self.on_build(|e| {
            if e.held.is_none() {
                return Did::default();
            }
            e.nav(if up { NavAction::Previous } else { NavAction::Next })
        });
    }

    pub fn build_edited(&mut self, text: &str) {
        self.on_build(|e| e.typed(text));
    }

    pub fn build_edit_done(&mut self, text: &str) {
        self.on_build(|e| e.typed_done(text));
    }
}

/// A save file, chosen in the system's dialog.
#[cfg(not(target_arch = "wasm32"))]
fn pick_save() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("save", &["sav", "raw", "srm"]).pick_file()
}

#[cfg(target_arch = "wasm32")]
fn pick_save() -> Option<PathBuf> {
    None
}

impl BuildsState {
    /// The builds of a game a player can choose (Play's, the lobby's):
    /// their names and sides, those its game reads.
    pub fn choices(content: &Content, game: &str) -> Vec<(String, Side)> {
        store::list(game)
            .into_iter()
            .filter_map(|l| store::read(content, game, &l.text).ok())
            .map(|(name, mut side)| {
                layout::as_built(content, game, &mut side);
                (name, side)
            })
            .collect()
    }
}
