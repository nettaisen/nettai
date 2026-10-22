//! The build overview's model (docs/design/app-ui.md §6; the UI's `BuildOverview`, build.slint): what a build
//! comes to, as a round starts it, and the whole of what it holds. Every number is the compiled one
//! (`nettai_match::build::Checked::overview`: the stats as the NaviCust and the patch cards leave them, with what they
//! added; after the eight, those the game's rules give, by their names in the game's strings), the abilities and the
//! bugs are named by the game's own strings, and the folder's chips have the pack's own icons. Each part of the build
//! is read as its editor's tab reads it (the choices' presets as the Navi tab's, the NaviCust's programs as placed,
//! the patch cards' list or slots, the auto battle data's lists, the partners' programs), every item of each: the
//! sheet shows them whole (the user: "when i hold tab to see the builds entire sections are missing like auto battle,
//! partycust specifics, etc.").
//!
//! Any screen shows a build with it: [`sheet`] makes the model of a side, and the screen sets it as the
//! `build` of a `BuildOverview` (docs/app.md §8).

use crate::builds::auto::{self, AutoBattle, Entry, Holds};
use crate::builds::words::{Words, kind};
use crate::builds::{Kit, cards, grid};
use crate::games::Names;
use crate::shell::Shell;
use nettai_content_api::{Data, EntryHandle, Registry};
use nettai_match::build::{self, Bug, Checked, NamedStat, Overview, Source, Stat};
use nettai_match::facts;
use nettai_match::ruleset::Item;
use nettai_match::{Folder, Side};
use nettai_ui::{BuildSheet, ChipCell, ChipMark, SheetForm, SheetList, SheetProgram, StatLine};
use slint::{ComponentHandle, ModelRc, VecModel};

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
    let party = crate::builds::party::sheet_rows(kit, names, side);
    // The presets of its choices, after the navi (EXE4's and EXE5's MegaMan's Light or Dark).
    let presets: Vec<String> = kit.builds.choices_of(content, side).iter().filter_map(|c| kit.builds.preset_of(content, side, &c.id).map(|p| crate::builds::preset_name(names, c, p))).collect();
    // What is the game's own navi's alone (its NaviCust, patch cards and auto battle data: the editor's tabs that are)
    // a build of another navi has none of.
    let own = content.navi(navi).changes_form();
    let navicust = own && facts::takes(content, grid::PIECES_FIELD);
    let programs: Vec<SheetProgram> = match navicust {
        true => grid::programs(content, side)
            .into_iter()
            .map(|(piece, color, copies)| SheetProgram { name: names.entry(EntryHandle(piece)).into(), tone: grid::tone(color.as_deref()), copies: copies as i32 })
            .collect(),
        false => Vec::new(),
    };
    let patch_cards = own && facts::takes(content, facts::PATCH_CARDS);
    let held = if patch_cards { cards::held(&kit.builds, content, side, facts::PATCH_CARDS) } else { Vec::new() };
    let card_names: Vec<String> = held.into_iter().flatten().map(|(h, _)| names.entry(h)).collect();
    // (A pattern's places aren't the build's: it states no pattern records, as the Auto battle tab shows none.)
    let auto_battle: Vec<SheetList> = match kit.auto_layout().filter(|_| own) {
        Some(layout) => {
            let data = AutoBattle::of_side(content, side);
            let chip = |place: usize| if let Entry::Chip(c) = data.places[place] { Some(names.chip(c)) } else { None };
            layout
                .lists
                .iter()
                .filter(|l| l.holds != Holds::Patterns)
                .map(|l| SheetList { name: auto::list_name(names, l).into(), chips: l.places().filter_map(&chip).collect::<Vec<_>>().join(", ").into() })
                .collect()
        }
        None => Vec::new(),
    };
    let face = checked.face().map(|f| kit.pictures.shown(&f)).unwrap_or_else(|| kit.pictures.navi(navi));
    let chips: Vec<ChipCell> = (0..folder.chips.len()).map(|i| chip_cell(kit, names, &folder, i, false)).collect();
    let one_line = |(name, extent): &(String, String)| if extent.is_empty() { name.clone() } else { format!("{name} {extent}") };
    BuildSheet {
        label: label.into(),
        name: name.into(),
        navi: names.navi(navi).into(),
        presets: presets.join(" · ").into(),
        face,
        forms: ModelRc::new(VecModel::from(forms)),
        party: ModelRc::new(VecModel::from(party)),
        stats: ModelRc::new(VecModel::from(stats)),
        abilities: abilities.join(", ").into(),
        bugs: bugs.iter().map(one_line).collect::<Vec<_>>().join(", ").into(),
        navicust,
        programs: ModelRc::new(VecModel::from(programs)),
        patch_cards,
        cards: card_names.join(", ").into(),
        auto_battle: ModelRc::new(VecModel::from(auto_battle)),
        count: folder.chips().count() as i32,
        chips: ModelRc::new(VecModel::from(chips)),
    }
}
