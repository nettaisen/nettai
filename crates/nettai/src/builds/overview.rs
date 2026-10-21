//! The build overview's model (docs/design/app-ui.md §6; the UI's `BuildOverview`, build.slint): what a build
//! comes to, as a round starts it. Every number is the compiled one (`nettai_match::build::Checked::overview`:
//! the stats as the NaviCust and the patch cards leave them, with what they added; after the eight, those the
//! game's rules give, by their names in the game's strings), the abilities and the bugs are named by the game's
//! own strings, and the folder's chips have the pack's own icons.
//!
//! Any screen shows a build with it: [`sheet`] makes the model of a side, and the screen sets it as the
//! `build` of a `BuildOverview` (docs/app.md §8).

use crate::builds::words::{Words, kind};
use crate::builds::{Kit, cards};
use crate::games::Names;
use crate::shell::Shell;
use nettai_content_api::{Data, EntryHandle, Registry};
use nettai_match::build::{self, Bug, Checked, NamedStat, Overview, Source, Stat};
use nettai_match::ruleset::Item;
use nettai_match::{Folder, Side};
use nettai_ui::{BuildSheet, ChipCell, ChipMark, SheetForm, SheetLine, SheetLineKind, StatLine};
use slint::{ComponentHandle, ModelRc, VecModel};

/// The lines the overview has room for under its eight stats, the two titles apart: a stat of the game's rules'
/// own after them takes one (its row is 22 tall, a line 20, and the sheet has 16 to spare under its last line).
const LINES: usize = 9;

/// The forms to a row of the wide sheet's forms (build.slint's `form-fit`, over the stats' column): each row takes a
/// line of the abilities and bugs (its 22 against a line's 20; the sheet has 16 to spare).
const FORMS_TO_A_ROW: usize = 6;

/// A folder's entry `entry` as a row shows it; `flagged`: the folder's rules have something against it.
pub fn chip_cell(kit: &Kit, names: &Names, folder: &Folder, entry: usize, flagged: bool) -> ChipCell {
    let Some(chip) = folder.chips.get(entry).copied().flatten() else { return ChipCell { flagged, ..Default::default() } };
    let record = kit.content().chip(chip.id);
    ChipCell {
        there: true,
        icon: kit.pictures.icon(chip.id),
        name: names.chip(chip.id).into(),
        kind: kind(record.class),
        mark: match build::mark(folder, entry) {
            Some(build::Mark::Regular) => ChipMark::Regular,
            Some(build::Mark::Tag1) => ChipMark::Tag1,
            Some(build::Mark::Tag2) => ChipMark::Tag2,
            None => ChipMark::None,
        },
        element: kit.pictures.chip_element(chip.id),
        code: chip.code.letter().to_string().into(),
        flagged,
        // (The editor marks a version's own chips: the overview doesn't.)
        version: Default::default(),
    }
}

/// The line of effect `index` of entry `entry` (a patch card's, as its game's card screen lists it); none: the
/// screen shows no line for it. An effect that holds effects of its own (a two-part card's half: what it does with
/// its other half in) is what they do, their lines together.
fn effect_line(kit: &Kit, names: &Names, entry: EntryHandle, index: usize) -> Option<String> {
    let c = names.content;
    let d = c.defs.definitions.get(Registry::Entry, &c.defs.entry(entry).key)?;
    let Data::List(effects) = d.spec.field(cards::FIELD) else { return None };
    let effect = effects.get(index)?;
    let named = cards::named_by(kit);
    let text = |key: &str| names.text(cards::TABLE, key);
    let inner: Vec<String> = match effect {
        Data::Map(fields) => fields.iter().filter_map(|(_, v)| if let Data::List(e) = v.field(cards::FIELD) { Some(e) } else { None }).flatten().filter_map(|e| cards::line(e, text, &named)).map(|l| l.text).collect(),
        _ => Vec::new(),
    };
    if !inner.is_empty() {
        return Some(inner.join(", "));
    }
    cards::line(effect, text, &named).map(|l| l.text)
}

/// What gives an ability or a bug, by name: a program's name, or a card's line.
pub fn source(words: &Words, source: Source) -> String {
    match source {
        Source::Entry(entry) => words.names.entry(entry),
        Source::Effect(entry, index) => effect_line(words.kit, words.names, entry, index).unwrap_or_else(|| words.names.entry(entry)),
    }
}

/// A stat of the game's rules' own, by its name in the game's strings ("Soul time").
pub fn stat_name(words: &Words, stat: &NamedStat) -> String {
    words.names.text(build::STATS_TABLE, &stat.key).unwrap_or_else(|| stat.key.clone())
}

/// A stat as the overview says it: its value, and what the build's installed parts added (nothing: none said). An
/// empty `label`: the one the view gives it by its place (HP, Attack, ...).
pub fn stat_line(label: String, stat: Stat) -> StatLine {
    StatLine { label: label.into(), value: stat.value.to_string().into(), added: if stat.added() == 0 { Default::default() } else { format!("{:+}", stat.added()).into() } }
}

/// A bug, by its full name in the game's strings and its level or amount (a card's drawback that is no bug of its
/// own kind: its line, and no level).
pub fn bug(words: &Words, bug: &Bug) -> (String, String) {
    match bug {
        Bug::Named { key, extent } => (words.names.text(build::BUGS_TABLE, key).unwrap_or_else(|| key.clone()), words.extent(*extent)),
        Bug::From(from) => (source(words, *from), String::new()),
    }
}

/// `items` in `room` lines: all of them, or as many as fit less one with how many more there are; none as one
/// line that says so.
fn fitted(words: &Words, items: Vec<SheetLine>, room: usize) -> Vec<SheetLine> {
    if items.is_empty() {
        return vec![SheetLine { kind: SheetLineKind::Nothing, ..Default::default() }];
    }
    if items.len() <= room {
        return items;
    }
    let shown = room.saturating_sub(1);
    let more = (items.len() - shown) as i32;
    let mut out: Vec<SheetLine> = items.into_iter().take(shown).collect();
    out.push(SheetLine { kind: SheetLineKind::More, text: words.ui.global::<nettai_ui::BuildWords>().invoke_more(more), ..Default::default() });
    out
}

/// The overview of `side`, a build named `name` (a player's name, in a match, under `label`: YOU, OPPONENT),
/// in the language shown.
pub fn sheet(shell: &Shell, kit: &Kit, label: &str, name: &str, side: &Side) -> BuildSheet {
    let ui = shell.ui();
    let graphics = kit.ready.graphics(shell.lang);
    let names = Names::of(kit.content(), &graphics);
    let words = Words { ui: &ui, kit, names: &names };
    let mut checked = Checked::of(kit.content(), &kit.game, side);
    sheet_of(&words, label, name, side, &mut checked)
}

/// [`sheet`], of a build already checked.
pub fn sheet_of(words: &Words, label: &str, name: &str, side: &Side, checked: &mut Checked) -> BuildSheet {
    let (kit, names) = (words.kit, words.names);
    let content = kit.content();
    let navi = side.navi(content);
    let folder = side.folder(content);
    let overview = checked.overview(content, side);
    let say = words.ui.global::<nettai_ui::BuildWords>();
    let stats: Vec<StatLine> = match &overview {
        Some(o) => {
            let line = |stat: Stat, megabytes: bool| StatLine { value: if megabytes { say.invoke_megabytes(stat.value) } else { stat.value.to_string().into() }, ..stat_line(String::new(), stat) };
            let Overview { hp, attack, rapid, charge, custom, mega, giga, regular_memory, .. } = *o;
            let named = o.stats.iter().map(|s| StatLine { label: stat_name(words, s).into(), ..line(s.stat, false) });
            [hp, attack, rapid, charge, custom, mega, giga].map(|s| line(s, false)).into_iter().chain([line(regular_memory, true)]).chain(named).collect()
        }
        // (The round doesn't start: nothing is known of its numbers.)
        None => vec![StatLine::default(); 8],
    };
    let abilities: Vec<String> = overview.iter().flat_map(|o| o.abilities.iter()).map(|&s| source(words, s)).collect();
    let bugs: Vec<(String, String)> = overview.iter().flat_map(|o| o.bugs.iter()).map(|b| bug(words, b)).collect();
    // The build's forms (the user: "is there a way to fit selected crosses and souls and beast form into the stats
    // view for a build?"): each fact of forms it chooses, in the setup's order (the side's version's form first), in
    // its own order (the battle's: the Cross window's, the soul button's), each with its version's mark; none for a
    // navi that doesn't change form.
    let forms: Vec<SheetForm> = match content.navi(navi).forms.is_some() {
        true => kit
            .builds
            .form_choices(content, side)
            .iter()
            .flat_map(|c| kit.builds.chosen_forms(content, side, &c.fact))
            .map(|f| SheetForm { face: kit.pictures.face(f), version: kit.version_tag_of(names, Registry::Form, &Item::of(f.0)) })
            .collect(),
        false => Vec::new(),
    };
    // Its partners (a party battle's: EXE5's), a row each under the forms.
    let party = crate::builds::party::sheet_rows(kit, names, |hp| words.ui.global::<nettai_ui::PartyView>().invoke_hp_word(hp), side, checked, overview.as_ref());
    // The lines under the stats (a line fewer for each stat after the eight, for each row of forms and for each
    // partner): the bugs keep four of them where both lists are long.
    let room = LINES.saturating_sub(stats.len() - 8).saturating_sub(forms.len().div_ceil(FORMS_TO_A_ROW)).saturating_sub(party.len()).max(2);
    let ability_lines: Vec<SheetLine> = abilities.iter().map(|a| SheetLine { kind: SheetLineKind::Ability, text: a.into(), ..Default::default() }).collect();
    let bug_lines: Vec<SheetLine> = bugs.iter().map(|(name, extent)| SheetLine { kind: SheetLineKind::Bug, text: name.into(), extent: extent.into() }).collect();
    let room_for_bugs = bug_lines.len().clamp(1, 4.min(room - 1));
    let room_for_abilities = ability_lines.len().clamp(1, room - room_for_bugs);
    let mut lines = vec![SheetLine { kind: SheetLineKind::Abilities, ..Default::default() }];
    lines.extend(fitted(words, ability_lines, room_for_abilities));
    lines.push(SheetLine { kind: SheetLineKind::Bugs, ..Default::default() });
    lines.extend(fitted(words, bug_lines, room - room_for_abilities));
    let face = checked.face().map(|f| kit.pictures.shown(&f)).unwrap_or_else(|| kit.pictures.navi(navi));
    let chips: Vec<ChipCell> = (0..folder.chips.len()).map(|i| chip_cell(kit, names, &folder, i, false)).collect();
    let one_line = |(name, extent): &(String, String)| if extent.is_empty() { name.clone() } else { format!("{name} {extent}") };
    BuildSheet {
        label: label.into(),
        name: name.into(),
        navi: names.navi(navi).into(),
        face,
        forms: ModelRc::new(VecModel::from(forms)),
        party: ModelRc::new(VecModel::from(party)),
        stats: ModelRc::new(VecModel::from(stats)),
        lines: ModelRc::new(VecModel::from(lines)),
        abilities: abilities.join(", ").into(),
        bugs: bugs.iter().map(one_line).collect::<Vec<_>>().join(", ").into(),
        count: folder.chips().count() as i32,
        chips: ModelRc::new(VecModel::from(chips)),
    }
}
