//! The build overview's model (docs/design/app-ui.md §6; the UI's `BuildOverview`, build.slint): what a build
//! comes to, as a round starts it. Every number is the compiled one (`nettai_match::build::Checked::overview`:
//! the stats as the NaviCust and the patch cards leave them, with what they added), the abilities and the bugs
//! are named by the game's own strings, and the folder's chips have the pack's own icons.
//!
//! Any screen shows a build with it: [`sheet`] makes the model of a side, and the screen sets it as the
//! `build` of a `BuildOverview` (docs/app.md §8).

use crate::builds::words::{Words, kind};
use crate::builds::{Kit, cards};
use crate::games::Names;
use crate::shell::Shell;
use nettai_content_api::{Data, EntryHandle, Registry};
use nettai_match::build::{self, Bug, Checked, Overview, Source, Stat};
use nettai_match::{Folder, Side};
use nettai_ui::{BuildSheet, ChipCell, ChipMark, SheetLine, SheetLineKind, StatLine};
use slint::{ComponentHandle, ModelRc, VecModel};

/// The lines the overview has room for under its stats, the two titles apart.
const LINES: usize = 9;

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
        element: kit.pictures.element(record.family),
        code: chip.code.letter().to_string().into(),
        flagged,
    }
}

/// The line of effect `index` of entry `entry` (a patch card's, as its game's card screen lists it); none: the
/// screen shows no line for it.
fn effect_line(names: &Names, entry: EntryHandle, index: usize) -> Option<String> {
    let c = names.content;
    let d = c.defs.definitions.get(Registry::Entry, &c.defs.entry(entry).key)?;
    let Data::List(effects) = d.spec.field(cards::FIELD) else { return None };
    cards::line(effects.get(index)?, |key| names.text(cards::TABLE, key)).map(|l| l.text)
}

/// What gives an ability or a bug, by name: a program's name, or a card's line.
fn source(names: &Names, source: Source) -> String {
    match source {
        Source::Entry(entry) => names.entry(entry),
        Source::Effect(entry, index) => effect_line(names, entry, index).unwrap_or_else(|| names.entry(entry)),
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
            let line = |stat: Stat, megabytes: bool| StatLine {
                value: if megabytes { say.invoke_megabytes(stat.value) } else { stat.value.to_string().into() },
                added: if stat.added() == 0 { Default::default() } else { format!("{:+}", stat.added()).into() },
            };
            let Overview { hp, attack, rapid, charge, custom, mega, giga, regular_memory, .. } = *o;
            [hp, attack, rapid, charge, custom, mega, giga].map(|s| line(s, false)).into_iter().chain([line(regular_memory, true)]).collect()
        }
        // (The round doesn't start: nothing is known of its numbers.)
        None => vec![StatLine::default(); 8],
    };
    let abilities: Vec<String> = overview.iter().flat_map(|o| o.abilities.iter()).map(|&s| source(names, s)).collect();
    let bugs: Vec<(String, String)> = overview
        .iter()
        .flat_map(|o| o.bugs.iter())
        .map(|bug| match bug {
            Bug::Named { key, extent } => (names.text(build::BUGS_TABLE, key).unwrap_or_else(|| key.clone()), words.extent(*extent)),
            Bug::From(from) => (source(names, *from), String::new()),
        })
        .collect();
    // The lines under the stats: the bugs keep four of them where both lists are long.
    let ability_lines: Vec<SheetLine> = abilities.iter().map(|a| SheetLine { kind: SheetLineKind::Ability, text: a.into(), ..Default::default() }).collect();
    let bug_lines: Vec<SheetLine> = bugs.iter().map(|(name, extent)| SheetLine { kind: SheetLineKind::Bug, text: name.into(), extent: extent.into() }).collect();
    let room_for_bugs = bug_lines.len().clamp(1, 4);
    let room_for_abilities = ability_lines.len().clamp(1, LINES - room_for_bugs);
    let mut lines = vec![SheetLine { kind: SheetLineKind::Abilities, ..Default::default() }];
    lines.extend(fitted(words, ability_lines, room_for_abilities));
    lines.push(SheetLine { kind: SheetLineKind::Bugs, ..Default::default() });
    lines.extend(fitted(words, bug_lines, LINES - room_for_abilities));
    let face = checked.face().map(|f| kit.pictures.shown(&f)).unwrap_or_else(|| kit.pictures.navi(navi));
    let chips: Vec<ChipCell> = (0..folder.chips.len()).map(|i| chip_cell(kit, names, &folder, i, false)).collect();
    let one_line = |(name, extent): &(String, String)| if extent.is_empty() { name.clone() } else { format!("{name} {extent}") };
    BuildSheet {
        label: label.into(),
        name: name.into(),
        navi: names.navi(navi).into(),
        face,
        version: kit.version_tag(names, side),
        stats: ModelRc::new(VecModel::from(stats)),
        lines: ModelRc::new(VecModel::from(lines)),
        abilities: abilities.join(", ").into(),
        bugs: bugs.iter().map(one_line).collect::<Vec<_>>().join(", ").into(),
        count: folder.chips().count() as i32,
        chips: ModelRc::new(VecModel::from(chips)),
    }
}
