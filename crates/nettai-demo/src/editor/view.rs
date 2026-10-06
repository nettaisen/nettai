//! What the editor shows: a bar of file actions, the panes (the arena, with
//! the match's game, then each side's navi, folder, the lists its game's
//! rules take of it (`crate::editor::facts`: EXE6's Crosses, EXE5's souls),
//! auto battle data, patch cards and stats; only what the game's rules
//! have, every list the game's), and the problems, live.

use crate::editor::app::{App, Choice, Editor, Msg, Tab};
use crate::editor::names::Lang;
use iced::widget::{Column, Row, button, checkbox, column, container, image, pick_list, row, rule, scrollable, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};
use nettai_battle::content::{ChipClass, ChipFlags};
use nettai_match::stats;

pub const SIDES: [&str; 2] = ["Left (you)", "Right"];
pub(crate) const RED: Color = Color::from_rgb(0.85, 0.2, 0.2);
pub(crate) const GREEN: Color = Color::from_rgb(0.15, 0.6, 0.25);
pub(crate) const DIM: Color = Color::from_rgb(0.5, 0.5, 0.55);

pub(crate) fn heading<'a>(s: impl text::IntoFragment<'a>) -> Element<'a, Msg> {
    text(s).size(20).into()
}

fn label<'a>(s: impl text::IntoFragment<'a>) -> Element<'a, Msg> {
    text(s).size(14).width(Length::Fixed(160.0)).into()
}

fn field<'a>(name: impl text::IntoFragment<'a>, widget: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    row![label(name), widget.into()].spacing(8).align_y(Alignment::Center).into()
}

/// [`nav`], for a name made up as the view is.
fn nav_owned<'a>(name: String, tab: Tab, now: Tab) -> Element<'a, Msg> {
    let b = button(text(name).size(14)).width(Length::Fill).on_press(Msg::Tab(tab));
    b.style(if tab == now { button::primary } else { button::text }).into()
}

fn nav<'a>(name: &'a str, tab: Tab, now: Tab) -> Element<'a, Msg> {
    let b = button(text(name).size(14)).width(Length::Fill).on_press(Msg::Tab(tab));
    b.style(if tab == now { button::primary } else { button::text }).into()
}

pub(crate) fn icon<'a>(e: &'a Editor, chip: nettai_content_api::ChipHandle) -> Element<'a, Msg> {
    let key = &e.content.defs.chip(chip).key;
    match e.pictures.chip(key).and_then(|p| p.icon.clone()) {
        Some(h) => image(h).width(16).height(16).filter_method(image::FilterMethod::Nearest).into(),
        None => space().width(16).height(16).into(),
    }
}

/// The window: the match being edited, or the choice of a new match's game.
pub fn window(a: &App) -> Element<'_, Msg> {
    match &a.editor {
        Some(e) => view(e),
        None => choose(a),
    }
}

/// Before there is a match: which game it is of. Nothing is selected: a
/// match is of the game chosen for it, or of the file opened.
fn choose(a: &App) -> Element<'_, Msg> {
    let mut games = Row::new().spacing(12);
    for game in &a.games {
        games = games.push(button(text(game_label(game)).size(18)).padding(14).on_press(Msg::Choose(game.clone())));
    }
    let mut col = Column::new().spacing(16).push(heading("A new match")).push(text("Which game is it of?"));
    col = if a.games.is_empty() {
        col.push(text("No game has its pack: extract one into the packs directory ($NETTAI_PACKS, else data), or give it with --pack.").color(RED))
    } else {
        col.push(games)
    };
    col = col.push(row![text("Or").size(14), button("Open").on_press(Msg::Open), text("a match file: it is of the game it names.").size(14)].spacing(8).align_y(Alignment::Center));
    col = col.push(text(a.status.as_str()).size(13).color(RED));
    container(col).padding(24).into()
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
        // (The lists the game's rules take of a side, each a pane: EXE6's
        // Crosses, where the navi has forms to list; EXE5's souls.)
        for (index, title) in crate::editor::facts::lists(&e.content, e.m.game(), side) {
            tabs = tabs.push(nav_owned(format!("  {title}"), Tab::List(s, index), e.tab));
        }
        // (Patch cards are the navi's that changes form: the cards change
        // MegaMan's stats.)
        let changes_form = e.content.navi(side.navi(&e.content)).forms.is_some();
        // (Where the game's rules have auto battle: EXE5's.)
        if nettai_match::auto_battle::has(&e.content) {
            tabs = tabs.push(nav("  Auto battle", Tab::AutoBattle(s), e.tab));
        }
        if changes_form && nettai_match::has_patch_cards(&e.content) {
            tabs = tabs.push(nav("  Patch cards", Tab::Cards(s), e.tab));
        }
        if nettai_match::has_navicust(&e.content) && changes_form {
            tabs = tabs.push(nav("  NaviCust", Tab::NaviCust(s), e.tab));
        }
        tabs = tabs.push(nav("  Stats", Tab::Stats(s), e.tab));
    }

    let pane: Element<Msg> = match e.tab {
        Tab::Arena => arena(e),
        Tab::Navi(s) => navi(e, s),
        Tab::Folder(s) => folder(e, s),
        Tab::List(s, index) => crate::editor::facts::list(e, s, index),
        Tab::AutoBattle(s) => crate::editor::auto_battle::view(e, s),
        Tab::Cards(s) => cards(e, s),
        Tab::NaviCust(s) => crate::editor::navicust::view(e, s),
        Tab::Stats(s) => stats_pane(e, s),
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
    ]
    .spacing(10);
    col = col.push(rule::horizontal(1));
    col = col.push(place(0, &m.arena.first));
    col = col.push(checkbox(same).label("The set's later rounds on the same place").on_toggle(Msg::LaterSame));
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
    let navi = Choice { label: e.names.navi(c, side.navi(c)), value: side.navi(c) };
    let level = e.typed.get(&(s, "level")).cloned().unwrap_or(side.level(c).map_or(String::new(), |l| l.to_string()));
    let mut col = column![heading(SIDES[s]), field("Navi", pick_list(navis, Some(navi), move |n| Msg::Navi(s, n)))].spacing(10);
    // (No navi code for EXE5's MegaMan.)
    let level_kind = side.takes_level(c).then(|| c.navi(side.navi(c)).forms.is_none());
    if level_kind == Some(true) {
        col = col.push(field("Navi level", text_input("0", &level).on_input(move |t| Msg::Level(s, t)).width(Length::Fixed(80.0))));
        if let Some(last) = nettai_match::story::max_level(c, side.navi(c)) {
            col = col.push(
                text(format!("0 to {last}: changing it fills in the HP the story gives at that level (the stats pane); at {last}, the story done."))
                    .size(13)
                    .color(DIM),
            );
        } else if let Some(levels) = c.navi(side.navi(c)).levels.as_ref().filter(|_| crate::editor::levels::has_levels(c, side)) {
            let last = levels.by_level.len().saturating_sub(1);
            col = col.push(text(format!("0 to {last}: the round gives the navi the stats its save's reload gives at that level, the game cleared (the stats pane).")).size(13).color(DIM));
        }
    } else if level_kind == Some(false) {
        col = col.push(field("Navi code level", text_input("none", &level).on_input(move |t| Msg::Level(s, t)).width(Length::Fixed(80.0))));
        let last = c.navi(side.navi(c)).levels.as_ref().map_or(0, |l| l.by_level.len().saturating_sub(1));
        col = col.push(
            text(format!("Empty: no navi code (as usual). 0 to {last}: MegaMan received from a navi code, his level's gains over his NaviCust, no Beast Out button."))
                .size(13)
                .color(DIM),
        );
    }
    // What the game's rules take of the side, each by its setup field's
    // type (`crate::editor::facts`): EXE6's version (nothing chosen for a new
    // side: none is a default), EXE5's karma. The lists have their panes.
    col = col.push(rule::horizontal(1));
    col = col.push(text("What the rules take").size(16));
    col = col.push(crate::editor::facts::rows(e, s));
    col = col.push(rule::horizontal(1));
    col = col.push(button("Import from save…").on_press(Msg::ImportSave(s)));
    col = col.push(
        text(
            "From an EXE6 .sav: the version, Beast Out and the Crosses it owns, the navi code's level and the SP times. \
             From an EXE5 .sav (or a raw save image): its karma, the souls it has (its version's), its NaviCust board's size \
             and what a navi in auto battle plays from it. \
             A save of another game than the match's makes a new match of its game.",
        )
        .size(13)
        .color(DIM),
    );
    if nettai_match::Side::takes_sp_times(c) {
        col = col.push(rule::horizontal(1));
        col = col.push(sp_times(e, s));
    }
    col = col.push(rule::horizontal(1));
    col = col.push(round_stats(e, s));
    scrollable(col).into()
}

/// The side's SP navi deletion times (`mm:ss.cc`; empty the fastest), each
/// by the SP navi chip whose damage goes by it: the entries of the side's
/// fact the engine knows as `PlayerFact::SpTimes` (the rules' default lists
/// every SP chip of the game), in the library's order.
fn sp_times(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &*e.content;
    let side = e.side(s);
    let times = side.facts.sp_times(c);
    let mut col = column![text("SP navi deletion times").size(16), text("mm:ss.cc; empty: the fastest. The SP navi chips' damage goes by them.").size(13).color(DIM)]
        .spacing(6);
    let mut read: Vec<(usize, nettai_content_api::ChipHandle)> = times.iter().enumerate().map(|(i, &(chip, _))| (i, chip)).collect();
    e.order.chips(c, &mut read);
    for (i, chip) in read {
        let label = e.names.chip(c, chip);
        let shown = e.sp_typed.get(&(s, i)).cloned().unwrap_or_else(|| match times[i].1 {
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
    let f = &side.folder(c);
    // The limits are the round's stats' (the rules check them).
    let stats = match &e.round {
        Ok(st) => st[s],
        Err(_) => nettai_match::Side::fresh_stats(c, side.navi(c)),
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
    e.order.chips(c, &mut pool);
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
    // (The chips are those the side's rules let a folder hold, which a round
    // that doesn't start can't say: a side without its version, for one.)
    let hint: Element<Msg> = match &e.round {
        Err(why) if e.pool[s].is_empty() => text(format!("No chips to list yet: {why}")).size(13).color(RED).into(),
        _ => text("A code puts the chip in the selected entry.").size(12).color(DIM).into(),
    };
    let right = column![
        row![picture, about].spacing(10),
        text_input("search chips", &e.search).on_input(Msg::Search),
        hint,
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

pub(crate) fn class_name(c: ChipClass) -> &'static str {
    match c {
        ChipClass::Standard => "Standard",
        ChipClass::Mega => "Mega",
        ChipClass::Giga => "Giga",
        _ => "other",
    }
}

pub(crate) fn class_letter(c: ChipClass) -> &'static str {
    match c {
        ChipClass::Standard => "S",
        ChipClass::Mega => "M",
        ChipClass::Giga => "G",
        _ => "?",
    }
}

// ---- A side's patch cards ------------------------------------------------------------------

fn cards(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let cards = crate::editor::app::cards_of(c, side);
    let mb: u32 = cards.iter().map(|&card| c.defs.patch_card(card).mb as u32).sum();
    let mut installed = Column::new().spacing(2);
    for (i, &card) in cards.iter().enumerate() {
        let d = c.defs.patch_card(card);
        installed = installed.push(
            row![
                text(e.names.patch_card(c, card)).size(14).width(Length::Fill),
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
        .filter(|&h| nettai_match::ids::in_game(c, e.m.game(), &c.defs.patch_card(h).key))
        .filter(|h| !cards.contains(h))
        .map(|h| (e.names.patch_card(c, h), h))
        .filter(|(n, _)| needle.is_empty() || n.to_lowercase().contains(&needle))
        .collect();
    e.order.patch_cards(c, &mut all);
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
    // (Past the list's MB the rules say so, with the other problems.)
    row![
        column![
            heading(format!("{}: patch cards", SIDES[s])),
            text(format!("{mb} MB used; they apply in this order.")).size(14).color(DIM),
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

/// The stats side `s`'s round starts with, by name: what its rules built on
/// its navi's fresh stats from what the side brings (the save's facts, its
/// level, its NaviCust, its patch cards). Nothing here is edited: a side
/// states what its save brings among its facts (the navi pane).
fn stats_pane(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let mut col = column![
        heading(format!("{}: stats", SIDES[s])),
        text("As the round starts them: what the rules build on the navi's fresh stats from what the side brings (what the save brings, among the navi pane's facts; its level; its NaviCust and patch cards).")
            .size(13)
            .color(DIM),
    ]
    .spacing(6);
    let stats = match &e.round {
        Ok(st) => st[s],
        Err(why) => {
            col = col.push(text(format!("The round doesn't start: {why}")).size(13).color(RED));
            return scrollable(col).into();
        }
    };
    for f in stats::FIELDS {
        let value = match stats::to_toml(c, (f.get)(&stats)) {
            toml::Value::String(v) => v,
            other => other.to_string(),
        };
        col = col.push(
            row![
                text(f.name).size(13).width(Length::Fixed(170.0)),
                text(value).size(13).width(Length::Fixed(200.0)),
                text(f.about).size(12).color(DIM)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    scrollable(col).into()
}

pub fn theme(_: &App) -> Theme {
    Theme::Light
}
