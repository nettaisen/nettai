//! What the editor shows: a bar of file actions, the panes (the arena, with
//! the match's game, then each side's navi, folder, Crosses,
//! patch cards and stats; only what the game's rules have, every list the
//! game's), and the problems, live.

use crate::app::{Choice, Editor, Msg, Tab};
use crate::names::Lang;
use iced::widget::{Column, Row, button, checkbox, column, container, image, pick_list, row, rule, scrollable, slider, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};
use nettai_battle::content::{ChipClass, ChipFlags};
use nettai_battle::custom::GameVersion;
use nettai_match::stats::{self, Kind, Value};
use nettai_match::{FORMS_SYSTEM, NAVICUST_SYSTEM, PATCH_CARDS_SYSTEM};

pub const SIDES: [&str; 2] = ["Left (you)", "Right"];
const RED: Color = Color::from_rgb(0.85, 0.2, 0.2);
const GREEN: Color = Color::from_rgb(0.15, 0.6, 0.25);
const DIM: Color = Color::from_rgb(0.5, 0.5, 0.55);

fn heading<'a>(s: impl text::IntoFragment<'a>) -> Element<'a, Msg> {
    text(s).size(20).into()
}

fn label<'a>(s: impl text::IntoFragment<'a>) -> Element<'a, Msg> {
    text(s).size(14).width(Length::Fixed(160.0)).into()
}

fn field<'a>(name: impl text::IntoFragment<'a>, widget: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    row![label(name), widget.into()].spacing(8).align_y(Alignment::Center).into()
}

fn nav<'a>(name: &'a str, tab: Tab, now: Tab) -> Element<'a, Msg> {
    let b = button(text(name).size(14)).width(Length::Fill).on_press(Msg::Tab(tab));
    b.style(if tab == now { button::primary } else { button::text }).into()
}

fn icon<'a>(e: &'a Editor, chip: nettai_content_api::ChipHandle) -> Element<'a, Msg> {
    let key = &e.content.defs.chip(chip).key;
    match e.pictures.chip(key).and_then(|p| p.icon.clone()) {
        Some(h) => image(h).width(16).height(16).filter_method(image::FilterMethod::Nearest).into(),
        None => space().width(16).height(16).into(),
    }
}

pub fn view(e: &Editor) -> Element<'_, Msg> {
    let bar = row![
        button("New").on_press(Msg::New),
        button("Open").on_press(Msg::Open),
        button("Save").on_press(Msg::Save),
        button("Save as").on_press(Msg::SaveAs),
        button("Random").on_press(Msg::Draw).style(button::secondary),
        text(e.file_name()).size(14).width(Length::Fill),
        pick_list(
            [Choice { label: "English".into(), value: Lang::En }, Choice { label: "日本語".into(), value: Lang::Ja }],
            Some(Choice { label: if e.lang == Lang::En { "English".into() } else { "日本語".into() }, value: e.lang }),
            |c| Msg::Lang(c.value)
        ),
        button(text("Play")).on_press(Msg::Play).style(button::success),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut tabs = Column::new().spacing(2).width(Length::Fixed(170.0));
    tabs = tabs.push(nav("Arena", Tab::Arena, e.tab));
    for (s, name) in SIDES.iter().enumerate() {
        let side = e.side(s);
        tabs = tabs.push(text(*name).size(13).color(DIM));
        tabs = tabs.push(nav("  Navi", Tab::Navi(s), e.tab));
        tabs = tabs.push(nav("  Folder", Tab::Folder(s), e.tab));
        if nettai_match::ruleset_has_system(&e.content, FORMS_SYSTEM) && e.content.navi(side.navi).forms.is_some() {
            tabs = tabs.push(nav("  Crosses", Tab::Crosses(s), e.tab));
        }
        if nettai_match::facts::takes(&e.content, nettai_match::facts::SOULS_FIELD) {
            tabs = tabs.push(nav("  Souls", Tab::Souls(s), e.tab));
        }
        if nettai_match::ruleset_has_system(&e.content, PATCH_CARDS_SYSTEM) {
            tabs = tabs.push(nav("  Patch cards", Tab::Cards(s), e.tab));
        }
        if nettai_match::ruleset_has_system(&e.content, NAVICUST_SYSTEM) && e.content.navi(side.navi).forms.is_some() {
            tabs = tabs.push(nav("  NaviCust", Tab::NaviCust(s), e.tab));
        }
        tabs = tabs.push(nav("  Stats", Tab::Stats(s), e.tab));
    }

    let pane: Element<Msg> = match e.tab {
        Tab::Arena => arena(e),
        Tab::Navi(s) => navi(e, s),
        Tab::Folder(s) => folder(e, s),
        Tab::Crosses(s) => crosses(e, s),
        Tab::Souls(s) => souls(e, s),
        Tab::Cards(s) => cards(e, s),
        Tab::NaviCust(s) => crate::navicust::view(e, s),
        Tab::Stats(s) => stats_pane(e, s, None),
    };

    let problems: Element<Msg> = if e.problems.is_empty() {
        text("Ready to play.").color(GREEN).into()
    } else {
        let list = e.problems.iter().fold(Column::new().spacing(2), |c, p| c.push(text(p.as_str()).size(13).color(RED)));
        scrollable(list).height(Length::Fixed(90.0)).into()
    };
    let footer = column![problems, text(e.status.as_str()).size(12).color(DIM)].spacing(4);

    let body = row![container(scrollable(tabs)).padding(4), rule::vertical(1), container(pane).padding(8).width(Length::Fill).height(Length::Fill)];
    container(column![bar, rule::horizontal(1), body.height(Length::Fill), rule::horizontal(1), footer].spacing(6))
        .padding(10)
        .into()
}

// ---- The arena ------------------------------------------------------------------------

fn arena(e: &Editor) -> Element<'_, Msg> {
    let c = &e.content;
    let m = &e.m;
    let game = m.game();
    // The game first: everything below it is the game's.
    let games: Vec<Choice<String>> =
        e.games.iter().map(|g| Choice { label: game_label(g), value: g.clone() }).collect();
    let picked = games.iter().find(|g| g.value == game).cloned();
    // (A game is its rules: their systems, by name.)
    let systems: Vec<&str> = nettai_match::systems(c).iter().map(|&h| nettai_match::ids::local(&c.defs.system(h).key)).collect();
    let stage_label = |s: nettai_content_api::StageHandle| nettai_match::ids::local(&c.defs.stage(s).key).to_string();
    let stages: Vec<Choice<_>> =
        nettai_match::link_battle_stages(c, game).into_iter().map(|s| Choice { label: stage_label(s), value: s }).collect();
    let mut backgrounds: Vec<Choice<Option<String>>> = vec![Choice { label: "the stage's own".into(), value: None }];
    // (By the name a match writes: the game's.)
    backgrounds.extend(nettai_match::ids::backgrounds(c, game).into_iter().map(|b| Choice { label: b.to_string(), value: Some(b.to_string()) }));
    let place = |i: usize, p: &nettai_match::Place| -> Element<Msg> {
        let stage = Choice { label: stage_label(p.stage), value: p.stage };
        let bg = Choice { label: p.background.clone().unwrap_or("the stage's own".into()), value: p.background.clone() };
        column![
            field("Stage", pick_list(stages.clone(), Some(stage), move |s| Msg::Stage(i, s))),
            field("Background", pick_list(backgrounds.clone(), Some(bg), move |b| Msg::Background(i, b))),
        ]
        .spacing(6)
        .into()
    };
    let same = m.arena.later == [m.arena.first.clone(), m.arena.first.clone()];
    let seed = e.typed.get(&(2, "seed")).cloned().unwrap_or_else(|| m.seed.map(|s| s.to_string()).unwrap_or_default());
    let mut col = column![
        heading("Arena"),
        field("Game", pick_list(games, picked, Msg::Game)),
        text("The match is of one game: both sides play by its rules, their navis, chips, souls and patch cards are its, and everything below lists its alone. Changing it starts the sides over.")
            .size(13)
            .color(DIM),
        text(format!("Its rules' systems: {}", if systems.is_empty() { "none".into() } else { systems.join(", ") })).size(13).color(DIM),
        rule::horizontal(1),
        place(0, &m.arena.first),
        checkbox(same).label("The set's later rounds on the same place").on_toggle(Msg::LaterSame),
    ]
    .spacing(10);
    if !same {
        for (i, p) in m.arena.later.iter().enumerate() {
            col = col.push(text(format!("Round {}", i + 2)).size(14));
            col = col.push(place(i + 1, p));
        }
    }
    col = col.push(field("Seed", text_input("from the clock", &seed).on_input(Msg::Seed).width(Length::Fixed(160.0))));
    scrollable(col).into()
}

// ---- A side's navi ----------------------------------------------------------------------

fn navi(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    // (The match's game's navis.)
    let navis: Vec<Choice<_>> = nettai_match::navis(c, e.m.game()).into_iter().map(|n| Choice { label: e.names.navi(c, n), value: n }).collect();
    let navi = Choice { label: e.names.navi(c, side.navi), value: side.navi };
    let versions = [Choice { label: "Falzar".into(), value: GameVersion::Falzar }, Choice { label: "Gregar".into(), value: GameVersion::Gregar }];
    let version = versions.iter().find(|g| g.value == side.game).cloned();
    let level = e.typed.get(&(s, "level")).cloned().unwrap_or(side.navi_level.map_or(String::new(), |l| l.to_string()));
    let frags = e.typed.get(&(s, "bug_frags")).cloned().unwrap_or(side.bug_frags.to_string());
    let mut col = column![heading(SIDES[s]), field("Navi", pick_list(navis, Some(navi), move |n| Msg::Navi(s, n)))].spacing(10);
    // What the rules and the navi take, alone: EXE6's version, a navi
    // code's level (`nettai_match::facts`).
    if nettai_match::Side::takes_game(c) {
        col = col.push(field("Version", pick_list(versions, version, move |g| Msg::Version(s, g))));
    }
    // (No navi code for EXE5's MegaMan.)
    let level_kind = side.takes_level(c).then(|| c.navi(side.navi).forms.is_none());
    if level_kind == Some(true) {
        col = col.push(field("Navi level", text_input("0", &level).on_input(move |t| Msg::Level(s, t)).width(Length::Fixed(80.0))));
        if crate::levels::has_levels(c, side) {
            col = col.push(text("0 to 14: changing it fills in the stats the save gives at that level, the game cleared (the stats pane).").size(13).color(DIM));
        }
    } else if level_kind == Some(false) {
        col = col.push(field("Navi code level", text_input("none", &level).on_input(move |t| Msg::Level(s, t)).width(Length::Fixed(80.0))));
        col = col.push(
            text("Empty: no navi code (as usual). 0 to 14: MegaMan received from a navi code, his level's gains over his NaviCust, no Beast Out button.")
                .size(13)
                .color(DIM),
        );
    }
    col = col.push(field("Bug frags", text_input("0", &frags).on_input(move |t| Msg::BugFrags(s, t)).width(Length::Fixed(100.0))));
    col = col.push(button("Import from save…").on_press(Msg::ImportSave(s)));
    col = col.push(
        text(
            "From an EXE6 .sav: the version, Beast Out and the Crosses it owns, the navi code's level and the SP times. \
             From an EXE5 .sav (or a raw save image): its karma, the souls it has (its version's) and its NaviCust board's size. \
             A save of another game than the match's makes a new match of its game.",
        )
        .size(13)
        .color(DIM),
    );
    if nettai_match::facts::takes(c, nettai_match::facts::KARMA_FIELD) {
        col = col.push(rule::horizontal(1));
        col = col.push(karma(e, s));
    }
    if nettai_match::Side::takes_sp_times(c) {
        col = col.push(rule::horizontal(1));
        col = col.push(sp_times(e, s));
    }
    col = col.push(rule::horizontal(1));
    col = col.push(round_stats(e, s));
    scrollable(col).into()
}

/// EXE5's karma, the save's light/dark value: a slider from 0 to 1000 and
/// its number, presets (light 500, very light 1000, dark 0), and what EXE5
/// makes of it (0x08010118: a dark MegaMan under 470, light from 470; the
/// starting mood's tiers, 0x0801283A: under 470 dark, under 500 worried,
/// 1000 the brightest; at or under 499 he clears holy panels).
fn karma(e: &Editor, s: usize) -> Element<'_, Msg> {
    let v = e.side(s).karma;
    let set = move |x: u16| Msg::Karma(s, x);
    let shown = e.typed.get(&(s, "karma")).cloned().unwrap_or_else(|| v.to_string());
    let kind = if v < 470 { "dark" } else if v >= 1000 { "very light" } else { "light" };
    let mood = match v {
        0..=469 => "0: the dark face and palette, dark chips usable in a link battle, light chips refused, no soul button".to_string(),
        470..=499 => "64: worried (no soul button until the mood rises)".to_string(),
        1000.. => "190, the palette's brightest tier".to_string(),
        _ => format!("{} (karma / 20 + 103; 0x80 at 500)", v / 20 + 103),
    };
    let holy = if v <= 499 { " Holy panels he stands on turn Normal." } else { "" };
    let preset = |name: &'static str, x: u16| button(text(format!("{name} ({x})")).size(13)).on_press(set(x)).style(button::secondary);
    column![
        text("Light and dark").size(16),
        row![
            label_text("Karma".into()),
            slider(0..=1000, v.min(1000), set).step(10u16).width(Length::Fixed(300.0)),
            text_input("500", &shown).on_input(move |t| Msg::KarmaText(s, t)).width(Length::Fixed(70.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        row![space().width(Length::Fixed(160.0)), preset("Light", 500), preset("Very light", 1000), preset("Dark", 0)].spacing(8),
        text(format!("{v}: a {kind} MegaMan. Starting mood {mood}.{holy}")).size(13),
        text("The save's light/dark value, 0 to 1000; a fresh save's 500. Under 470 dark (dark chips, no light ones); 499 or under clears holy panels; under 500 worried at the start; 1000 the brightest.")
            .size(13)
            .color(DIM),
    ]
    .spacing(6)
    .into()
}

fn label_text<'a>(s: String) -> Element<'a, Msg> {
    text(s).size(14).width(Length::Fixed(160.0)).into()
}

/// The side's SP navi deletion times (`mm:ss.cc`; empty the fastest), each
/// by the SP navi chip that reads it.
fn sp_times(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &*e.content;
    let side = e.side(s);
    let slots = nettai_match::sp_slots(c);
    let mut col = column![text("SP navi deletion times").size(16), text("mm:ss.cc; empty: the fastest. The SP navi chips' damage goes by them.").size(13).color(DIM)]
        .spacing(6);
    for (i, slot) in slots.iter().enumerate() {
        // The SP navi chip whose damage reads the slot (the game's), by
        // its name.
        let chip = nettai_match::facts::sp_chip(c, &e.m.arena, i);
        let label = chip.map_or_else(|| slot.clone(), |h| e.names.chip(c, h));
        let shown = e.sp_typed.get(&(s, i)).cloned().unwrap_or_else(|| match side.sp_times.0[i] {
            0 => String::new(),
            f => nettai_match::sp_times::format(f),
        });
        col = col.push(field(label, text_input("00:00.00", &shown).on_input(move |t| Msg::SpTime(s, i, t)).width(Length::Fixed(100.0))));
    }
    col.into()
}

/// What the round starts the navi with, once the rules have set it up.
pub fn round_stats(e: &Editor, s: usize) -> Element<'_, Msg> {
    match &e.round {
        Ok(stats) => {
            let st = &stats[s];
            let supports = match st.support {
                None => "the support bug".to_string(),
                Some(n) => {
                    let names: Vec<&str> = [(n.rush, "Rush"), (n.beat, "Beat"), (n.tango, "Tango")].iter().filter(|x| x.0).map(|x| x.1).collect();
                    if names.is_empty() { "none".into() } else { names.join(", ") }
                }
            };
            let b = &st.bugs;
            let bugs: Vec<String> = [
                (b.processing != 0, "steps go astray".to_string()),
                (b.panel_trail_level != 0, format!("panel trail {} at level {}", b.panel_trail_kind, b.panel_trail_level)),
                (b.buster_blanks != 0, format!("{} blank buster shots of 16", b.buster_blanks)),
                (b.hit_status != 0, format!("hits give status {}", b.hit_status)),
                (b.hp_drain != 0, format!("HP drain {}", b.hp_drain)),
                (b.custom_drain != 0, format!("custom screen HP drain {}", b.custom_drain)),
                (b.battle_start != 0, format!("battle start status {}", b.battle_start)),
                (b.emotion != 0, "the emotion swings".to_string()),
                (b.hand_shrink_turn != 0, format!("the hand shrinks from turn {}", b.hand_shrink_turn)),
                (b.custom_damage != 0, format!("{} damage as the custom screen opens", b.custom_damage)),
            ]
            .into_iter()
            .filter(|x| x.0)
            .map(|x| x.1)
            .collect();
            column![
                text("As the round starts (after the NaviCust and the patch cards)").size(15),
                text(format!(
                    "HP {} · Attack {} · Rapid {} · Charge {} · Custom {} · Mega {} · Giga {} · Regular {} MB",
                    st.max_hp,
                    st.attack + 1,
                    st.rapid + 1,
                    st.charge + 1,
                    st.custom_level,
                    st.mega_level,
                    st.giga_level,
                    st.reg_up
                ))
                .size(13),
                text(format!(
                    "SuprArmr {} · FlotShoe {} · AirShoes {} · UnderSht {} · supports: {supports}",
                    yes(st.super_armor),
                    yes(st.float_shoes),
                    yes(st.air_shoes),
                    yes(st.undershirt)
                ))
                .size(13),
                text(if bugs.is_empty() { "No bugs.".into() } else { format!("Bugs: {}", bugs.join("; ")) })
                    .size(13)
                    .color(if bugs.is_empty() { DIM } else { RED }),
            ]
            .spacing(4)
            .into()
        }
        Err(why) => text(why.as_str()).color(RED).into(),
    }
}

fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

// ---- A side's folder ----------------------------------------------------------------------

fn folder(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let f = &side.folder;
    // The limits are the stats' (the rules check them).
    let stats = match &e.round {
        Ok(st) => st[s],
        Err(_) => side.stats,
    };
    // The folder's entries.
    let mut entries = Column::new().spacing(1);
    for (i, chip) in f.chips.iter().enumerate() {
        let mut marks = String::new();
        if f.regular == Some(i as u8) {
            marks.push_str(" REG");
        }
        if f.tags.is_some_and(|(a, b)| a == i as u8 || b == i as u8) {
            marks.push_str(" TAG");
        }
        let line = match chip {
            Some(chip) => {
                let ok = c.chip(chip.id).codes.contains(&chip.code);
                row![
                    text(format!("{i:>2}")).size(12).color(DIM).width(Length::Fixed(22.0)),
                    icon(e, chip.id),
                    text(e.names.chip(c, chip.id)).size(14).width(Length::Fill),
                    text(chip.code.letter().to_string()).size(14).color(if ok { Color::BLACK } else { RED }).width(Length::Fixed(16.0)),
                    text(marks).size(12).color(GREEN).width(Length::Fixed(64.0)),
                ]
            }
            None => row![
                text(format!("{i:>2}")).size(12).color(DIM).width(Length::Fixed(22.0)),
                space().width(16).height(16),
                text("(empty)").size(14).color(DIM).width(Length::Fill),
                text(marks).size(12).color(GREEN).width(Length::Fixed(64.0)),
            ],
        }
        .spacing(6)
        .align_y(Alignment::Center);
        let b = button(line).width(Length::Fill).padding([1, 4]).on_press(Msg::Entry(s, i));
        entries = entries.push(b.style(if e.entry[s] == i { button::secondary } else { button::text }));
    }
    let (picture, about): (Element<Msg>, Element<Msg>) = match f.chips[e.entry[s]] {
        Some(selected) => {
            let picture = match e.pictures.chip(&c.defs.chip(selected.id).key).and_then(|p| p.art.clone()) {
                Some(h) => image(h).width(112).height(96).filter_method(image::FilterMethod::Nearest).into(),
                None => space().width(112).height(96).into(),
            };
            let sd = c.chip(selected.id);
            let about = column![
                text(format!("Entry {}: {} {}", e.entry[s], e.names.chip(c, selected.id), selected.code.letter())).size(15),
                text(format!("{} · {} MB · {} damage", class_name(sd.class), sd.mb, sd.damage))
                .size(13)
                .color(DIM),
                row![
                    button("Regular").on_press(Msg::Regular(s)).style(button::secondary),
                    button("Tag").on_press(Msg::Tag(s)).style(button::secondary),
                    button("Clear").on_press(Msg::ClearEntry(s)).style(button::secondary),
                ]
                .spacing(6),
            ]
            .spacing(4);
            (picture, about.into())
        }
        None => (
            space().width(112).height(96).into(),
            column![
                text(format!("Entry {}: empty", e.entry[s])).size(15),
                text("A code in the list below puts that chip here.").size(13).color(DIM),
            ]
            .spacing(4)
            .into(),
        ),
    };
    // The counts and the limits, live.
    let count = |class: ChipClass| f.chips().filter(|x| c.chip(x.id).class == class).count();
    let chip_at = |i: u8| f.chips.get(i as usize).copied().flatten();
    let regular = f.regular.and_then(chip_at).map(|x| {
        format!("Regular: {} ({} MB; the navi's memory {})", e.names.chip(c, x.id), c.chip(x.id).mb, stats.reg_up)
    });
    let tags = f.tags.map(|(a, b)| {
        let mb: u32 = [a, b].into_iter().filter_map(chip_at).map(|x| c.chip(x.id).mb as u32).sum();
        format!("Tags: entries {a} and {b} ({mb} MB)")
    });
    let counts = text(format!(
        "{} of {} chips · Mega {} (the navi's level {}) · Giga {} (level {}) · {} · {}",
        f.chips().count(),
        f.chips.len(),
        count(ChipClass::Mega),
        stats.mega_level,
        count(ChipClass::Giga),
        stats.giga_level,
        regular.unwrap_or("no Regular chip".into()),
        tags.unwrap_or("no tag chips".into()),
    ))
    .size(13);
    let left = column![heading(format!("{}: folder", SIDES[s])), counts, scrollable(entries).height(Length::Fill)]
        .spacing(6)
        .width(Length::FillPortion(1));
    // The chips a folder can hold (the game's the rules take), searched.
    let needle = e.search.to_lowercase();
    let mut pool: Vec<(String, nettai_content_api::ChipHandle)> = e.pool[s]
        .iter()
        .copied()
        .map(|h| (e.names.chip(c, h), h))
        .filter(|(name, h)| needle.is_empty() || name.to_lowercase().contains(&needle) || nettai_match::ids::local(&c.defs.chip(*h).key).contains(&needle))
        .collect();
    pool.sort();
    let mut list = Column::new().spacing(1);
    for (name, h) in pool.into_iter().take(400) {
        let d = c.chip(h);
        let held = f.chips().filter(|x| x.id == h).count();
        let codes = d.codes.iter().fold(Row::new().spacing(2), |r, &code| {
            r.push(button(text(code.letter().to_string()).size(12)).padding([1, 5]).on_press(Msg::Put(s, h, code)))
        });
        let count = if held > 0 { format!("×{held}") } else { String::new() };
        let dark = d.flags.has(ChipFlags::DARK);
        list = list.push(
            row![
                icon(e, h),
                text(name).size(14).width(Length::Fill).color(if dark { RED } else { Color::BLACK }),
                text(format!("{} {} MB", class_letter(d.class), d.mb)).size(12).color(DIM).width(Length::Fixed(64.0)),
                text(count).size(12).color(GREEN).width(Length::Fixed(34.0)),
                container(codes.wrap()).width(Length::Fixed(150.0)),
                space().width(Length::Fixed(12.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    let right = column![
        row![picture, about].spacing(10),
        text_input("search chips", &e.search).on_input(Msg::Search),
        text("A code puts the chip in the selected entry.").size(12).color(DIM),
        scrollable(list).height(Length::Fill),
    ]
    .spacing(6)
    .width(Length::FillPortion(1));
    row![left.width(Length::FillPortion(2)), right.width(Length::FillPortion(3))].spacing(12).into()
}

/// A game as the editor names it (`EXE5`).
fn game_label(game: &str) -> String {
    game.to_uppercase()
}

fn class_name(c: ChipClass) -> &'static str {
    match c {
        ChipClass::Standard => "Standard",
        ChipClass::Mega => "Mega",
        ChipClass::Giga => "Giga",
        _ => "other",
    }
}

fn class_letter(c: ChipClass) -> &'static str {
    match c {
        ChipClass::Standard => "S",
        ChipClass::Mega => "M",
        ChipClass::Giga => "G",
        _ => "?",
    }
}

// ---- A side's Crosses ------------------------------------------------------------------------

fn crosses(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let own = side.crosses.is_none();
    let mut col = column![
        heading(format!("{}: Crosses", SIDES[s])),
        checkbox(own).label("The version's own five (the save's)").on_toggle(move |b| Msg::OwnCrosses(s, b)),
    ]
    .spacing(8);
    if !own {
        let list: Vec<_> = side.crosses.map(|l| l.forms().collect()).unwrap_or_default();
        col = col.push(text(format!("{} of {} chosen; the window offers them in this order.", list.len(), nettai_battle::custom::screen::CROSSES)).size(13).color(DIM));
        for f in nettai_match::navi_crosses(c, side.navi).unwrap_or_default() {
            let on = list.contains(&f);
            let version = match exe6_compat::forms::game(c, f) {
                Some(GameVersion::Gregar) => "Gregar",
                Some(GameVersion::Falzar) => "Falzar",
                None => "",
            };
            col = col.push(checkbox(on).label(format!("{} ({version})", e.names.form(c, f))).on_toggle(move |b| Msg::Cross(s, f, b)));
        }
    }
    scrollable(col).into()
}

// ---- A side's souls ------------------------------------------------------------------------

/// The souls the side has (EXE5's Soul Unison): every soul of the match's
/// game (the default), or those checked, of either version. A soul whose
/// chip family the folder never holds never comes up.
fn souls(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let every = side.souls.is_none();
    let owned = nettai_match::facts::owned_souls(c, e.m.game(), side);
    let all = nettai_match::facts::all_souls(c, e.m.game());
    let mut col = column![
        heading(format!("{}: souls", SIDES[s])),
        text("The souls the soul button may offer (for the last chip picked of the soul's family). Any of the game's souls, either version's: a real save has its version's six.")
            .size(13)
            .color(DIM),
        checkbox(every).label("Every soul (the default)").on_toggle(move |b| Msg::EverySoul(s, b)),
        text(format!("{} of {} souls", owned.len(), all.len())).size(13).color(DIM),
    ]
    .spacing(8);
    for f in all {
        let form = c.form(f);
        let face: Element<Msg> = match e.pictures.face(&c.defs.form(f).key) {
            Some(h) => image(h.clone()).width(64).height(32).filter_method(image::FilterMethod::Nearest).into(),
            None => space().width(64).height(32).into(),
        };
        let about = form.soul.as_ref().map_or(String::new(), |x| format!("soul {}, for {:?} chips", x.number, x.family).to_lowercase());
        let on = owned.contains(&f);
        let mut tick = checkbox(on);
        if !every {
            tick = tick.on_toggle(move |b| Msg::Soul(s, f, b));
        }
        col = col.push(
            row![tick, face, text(e.names.form(c, f)).size(15).width(Length::Fixed(160.0)), text(about).size(13).color(DIM)]
                .spacing(10)
                .align_y(Alignment::Center),
        );
    }
    scrollable(col).into()
}

// ---- A side's patch cards ------------------------------------------------------------------

fn cards(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let mb: u32 = side.cards.iter().map(|x| c.defs.patch_card(x.card).mb as u32).sum();
    let mut installed = Column::new().spacing(2);
    for (i, card) in side.cards.iter().enumerate() {
        let d = c.defs.patch_card(card.card);
        installed = installed.push(
            row![
                checkbox(card.enabled).on_toggle(move |b| Msg::CardOn(s, i, b)),
                text(e.names.patch_card(c, card.card)).size(14).width(Length::Fill),
                text(format!("{} MB", d.mb)).size(12).color(DIM),
                button(text("↑").size(12)).on_press(Msg::CardMove(s, i, true)).style(button::text),
                button(text("↓").size(12)).on_press(Msg::CardMove(s, i, false)).style(button::text),
                button(text("remove").size(12)).on_press(Msg::CardRemove(s, i)).style(button::danger),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }
    let needle = e.search.to_lowercase();
    // (The match's game's.)
    let mut all: Vec<(String, nettai_content_api::PatchCardHandle)> = (0..c.defs.patch_cards.len() as u16)
        .map(nettai_content_api::PatchCardHandle)
        .filter(|&h| nettai_match::ids::in_game(e.m.game(), &c.defs.patch_card(h).key))
        .filter(|h| !side.cards.iter().any(|x| x.card == *h))
        .map(|h| (e.names.patch_card(c, h), h))
        .filter(|(n, _)| needle.is_empty() || n.to_lowercase().contains(&needle))
        .collect();
    all.sort();
    let available = all.into_iter().fold(Column::new().spacing(1), |col, (name, h)| {
        let d = c.defs.patch_card(h);
        let bugs = d.effects.iter().filter(|x| x.bug).count();
        col.push(
            row![
                button(text("add").size(12)).on_press(Msg::AddCard(s, h)).style(button::secondary),
                text(name).size(14).width(Length::Fill),
                text(format!("{} MB · {} effects{}", d.mb, d.effects.len(), if bugs > 0 { format!(", {bugs} bugs") } else { String::new() }))
                    .size(12)
                    .color(DIM),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
    });
    let over = mb > nettai_match::check::CARD_MB;
    row![
        column![
            heading(format!("{}: patch cards", SIDES[s])),
            text(format!("{mb} MB of {} used; they apply in this order.", nettai_match::check::CARD_MB)).size(14).color(if over { RED } else { DIM }),
            scrollable(installed).height(Length::Fill),
        ]
        .spacing(8)
        .width(Length::FillPortion(1)),
        column![text_input("search cards", &e.search).on_input(Msg::Search), scrollable(available).height(Length::Fill)]
            .spacing(6)
            .width(Length::FillPortion(1)),
    ]
    .spacing(12)
    .into()
}

// ---- A side's stats --------------------------------------------------------------------------

/// What a NaviCust gives the navi: its stats and bugs, set directly.
const NAVICUST_FIELDS: &[&str] = &[
    "hp", "attack", "rapid", "charge", "custom_level", "mega_level", "giga_level", "super_armor", "float_shoes", "air_shoes",
    "undershirt", "first_barrier", "back_special", "supports", "chip_shuffle", "number_open", "step_bug", "panel_trail",
    "panel_trail_level", "buster_blanks", "buster_charged", "hit_status", "hp_drain", "emotion_bug", "battle_start_bug",
    "hand_shrink_turn",
];

/// The NaviCust's stats set directly (no grid of programs).
pub fn navicust_stats(e: &Editor, s: usize) -> Element<'_, Msg> {
    stats_pane(e, s, Some(NAVICUST_FIELDS))
}

fn stats_pane<'a>(e: &'a Editor, s: usize, only: Option<&'static [&'static str]>) -> Element<'a, Msg> {
    let c = &e.content;
    let side = e.side(s);
    // (Weapons, records and forms by their names in the match's game.)
    let game = e.m.game();
    let local = |key: &str| nettai_match::ids::local(key).to_string();
    let ours = move |key: &str| nettai_match::ids::in_game(game, key);
    let base = crate::levels::reset(c, side);
    let leveled = crate::levels::has_levels(c, side);
    let (title, about) = match only {
        Some(_) => (String::new(), "What the NaviCust gives the navi, as its stats and bugs, set directly; a changed one is written to the file."),
        None if leveled => (
            format!("{}: stats", SIDES[s]),
            "What the save gives the link navi at its level (its reload); a changed one is written to the file, and where it differs from the level's, said.",
        ),
        None => (
            format!("{}: stats", SIDES[s]),
            "What the save and the NaviCust give the navi, over its fresh stats; a changed one is written to the file.",
        ),
    };
    let reset = if leveled { "Reset to the level's" } else { "Reset to fresh" };
    let mut col = column![
        row![heading(title), space().width(Length::Fill), button(reset).on_press(Msg::StatsReset(s)).style(button::secondary)]
            .align_y(Alignment::Center),
        text(about).size(13).color(DIM),
    ]
    .spacing(6);
    for f in stats::FIELDS.iter().filter(|f| only.is_none_or(|names| names.contains(&f.name))) {
        let now = (f.get)(&side.stats);
        let changed = now != (f.get)(&base);
        let name = text(f.name).size(13).width(Length::Fixed(170.0)).color(if changed { Color::BLACK } else { DIM });
        let widget: Element<Msg> = match (f.kind, now) {
            (Kind::Int(_), Value::Int(v)) => {
                let typed = e.typed.get(&(s, f.name)).cloned().unwrap_or(v.to_string());
                let name = f.name;
                text_input("", &typed).on_input(move |t| Msg::StatText(s, name, t)).width(Length::Fixed(90.0)).into()
            }
            (Kind::Bool, Value::Bool(v)) => {
                let name = f.name;
                checkbox(v).on_toggle(move |b| Msg::StatValue(s, name, Value::Bool(b))).into()
            }
            (Kind::Weapon, Value::Weapon(w)) => {
                let mut options = vec![Choice { label: "none".into(), value: None }];
                options.extend(
                    c.defs.weapons.iter().enumerate().filter(|(_, d)| ours(&d.key)).map(|(i, d)| Choice { label: local(&d.key), value: Some(nettai_content_api::WeaponHandle(i as u16)) }),
                );
                let now = options.iter().find(|o| o.value == w).cloned();
                let name = f.name;
                pick_list(options, now, move |o: Choice<_>| Msg::StatValue(s, name, Value::Weapon(o.value))).text_size(13).into()
            }
            (Kind::Record(ty), Value::Record(r)) => {
                let mut options = vec![Choice { label: "none".into(), value: None }];
                options.extend(
                    c.defs.records.iter().enumerate().filter(|(_, d)| d.record_type == ty && ours(&d.key)).map(|(i, d)| Choice {
                        label: local(&d.key),
                        value: Some(nettai_content_api::RecordHandle(i as u16)),
                    }),
                );
                let now = options.iter().find(|o| o.value == r).cloned();
                let name = f.name;
                pick_list(options, now, move |o: Choice<_>| Msg::StatValue(s, name, Value::Record(o.value))).text_size(13).into()
            }
            (Kind::Form, Value::Form(form)) => {
                let options: Vec<Choice<_>> = (0..c.defs.forms.len() as u16)
                    .map(nettai_content_api::FormHandle)
                    .filter(|&h| ours(&c.defs.form(h).key))
                    .map(|h| Choice { label: local(&c.defs.form(h).key), value: h })
                    .collect();
                let now = options.iter().find(|o| o.value == form).cloned();
                let name = f.name;
                pick_list(options, now, move |o: Choice<_>| Msg::StatValue(s, name, Value::Form(o.value))).text_size(13).into()
            }
            (Kind::Gauge, Value::Gauge(g)) => {
                use nettai_battle::setup::GaugeSpeed;
                let options: Vec<Choice<GaugeSpeed>> = [GaugeSpeed::Normal, GaugeSpeed::Fast, GaugeSpeed::Slow]
                    .into_iter()
                    .map(|g| Choice { label: stats::gauge_name(g).into(), value: g })
                    .collect();
                let now = options.iter().find(|o| o.value == g).cloned();
                let name = f.name;
                pick_list(options, now, move |o: Choice<GaugeSpeed>| Msg::StatValue(s, name, Value::Gauge(o.value))).text_size(13).into()
            }
            (Kind::Supports, Value::Supports(n)) => {
                let name = f.name;
                let n0 = n.unwrap_or_default();
                let set = move |n: Option<nettai_battle::setup::Supports>| Msg::StatValue(s, name, Value::Supports(n));
                row![
                    checkbox(n0.rush).label("Rush").on_toggle(move |b| set(Some(nettai_battle::setup::Supports { rush: b, ..n0 }))),
                    checkbox(n0.beat).label("Beat").on_toggle(move |b| set(Some(nettai_battle::setup::Supports { beat: b, ..n0 }))),
                    checkbox(n0.tango).label("Tango").on_toggle(move |b| set(Some(nettai_battle::setup::Supports { tango: b, ..n0 }))),
                    checkbox(n.is_none()).label("bug").on_toggle(move |b| set(if b { None } else { Some(n0) })),
                ]
                .spacing(8)
                .into()
            }
            _ => text("?").into(),
        };
        let mut line = row![name, widget].spacing(8).align_y(Alignment::Center);
        if let Some(note) = crate::levels::differs(c, side, f) {
            line = line.push(text(note).size(12).color(RED));
        }
        col = col.push(line.push(text(f.about).size(12).color(DIM)));
    }
    scrollable(col).into()
}

pub fn theme(_: &Editor) -> Theme {
    Theme::Light
}
