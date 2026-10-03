//! The NaviCust pane: a side's programs on the 7x7 grid as the game draws
//! them (the board of its expansions, its frame, the command line), each in
//! its color and shape, turned and compressed; programs added from the
//! list, picked on the grid, moved, turned, recolored, compressed, taken
//! off. What they give the navi (the stats and the bugs) is the rules'
//! compile, shown as the round starts it; what's wrong (off the board, over
//! another) the checks'.

use crate::app::{Choice, Editor, Msg};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Column, button, canvas as canvas_widget, checkbox, column, pick_list, row, scrollable, space, text, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use nettai_battle::Content;
use nettai_battle::content::BoardCell;
use nettai_battle::navicust::{NaviCust, PlacedProgram, SIZE, cells};
use nettai_content_api::NaviCustProgramHandle;
use nettai_match::Side;

/// A cell's size on the screen.
const CELL: f32 = 46.0;

/// What the pane keeps of its own: the program picked on the grid, and the
/// list's search.
#[derive(Clone, Debug, Default)]
pub struct State {
    pub selected: Option<usize>,
    pub search: String,
}

#[derive(Clone, Debug)]
pub enum Edit {
    /// A grid of programs, or the stats set directly.
    UseGrid(bool),
    Expansions(u8),
    Add(NaviCustProgramHandle, u8),
    /// A grid cell clicked: the program on it picked, or the picked one
    /// moved there.
    Cell(u8, u8),
    Move(i8, i8),
    Rotate,
    Compress(bool),
    Color(u8),
    Remove,
    Search(String),
}

/// A program color's look (the NaviCust's, as Tango draws them).
fn color(name: &str) -> Color {
    let (r, g, b) = match name {
        "white" => (0xDE, 0xDE, 0xDE),
        "yellow" => (0xDE, 0xDE, 0x00),
        "pink" => (0xDE, 0x8C, 0xC6),
        "red" => (0xDE, 0x10, 0x00),
        "blue" => (0x29, 0x84, 0xDE),
        "green" => (0x18, 0xC6, 0x00),
        "orange" => (0xDE, 0x7B, 0x00),
        "purple" => (0x94, 0x00, 0xCE),
        _ => (0x84, 0x84, 0x84),
    };
    Color::from_rgb8(r, g, b)
}

/// The program covering grid cell (x, y), by its place in the list.
fn at(content: &Content, n: &NaviCust, x: i32, y: i32) -> Option<usize> {
    // (The last placed on top, as the grid keeps it.)
    let parts: Vec<PlacedProgram> = n.iter().collect();
    parts.iter().enumerate().rev().find_map(|(i, p)| {
        let shape = content.navicust_program(p.program).placed_shape(p.compressed, p.rotation);
        cells(&shape, p.x, p.y).any(|c| c == (x, y)).then_some(i)
    })
}

fn set(side: &mut Side, parts: Vec<PlacedProgram>, expansions: u8) {
    side.navicust = NaviCust::new(&parts, expansions).ok().or(side.navicust);
}

/// Apply an edit; whether the match changed.
pub fn update(content: &Content, side: &mut Side, state: &mut State, edit: Edit) -> bool {
    let rules = nettai_match::navicust_rules(content, side).clone();
    let largest = rules.boards.len().saturating_sub(1) as u8;
    if let Edit::UseGrid(on) = edit {
        if on {
            // With a grid the stats set are only what the save keeps.
            let kept: std::collections::BTreeMap<String, toml::Value> =
                side.stats_block(content).into_iter().filter(|(k, _)| nettai_match::stats::SAVE_FIELDS.contains(&k.as_str())).collect();
            side.stats = Side::base_stats(content, side.navi, side.game);
            nettai_match::stats::apply(content, &kept, &mut side.stats);
            side.stats = nettai_match::starting(content, side.stats, side.game);
            side.navicust = Some(NaviCust::new(&[], largest).expect("an empty NaviCust"));
        } else {
            side.navicust = None;
        }
        state.selected = None;
        return true;
    }
    if let Edit::Search(t) = edit {
        state.search = t;
        return false;
    }
    let Some(n) = side.navicust else { return false };
    let mut parts: Vec<PlacedProgram> = n.iter().collect();
    let expansions = n.expansions;
    match edit {
        Edit::Expansions(x) => {
            set(side, parts, x);
            return true;
        }
        Edit::Add(program, color) => {
            let def = content.navicust_program(program);
            let shape = def.placed_shape(false, 0);
            let board = rules.board(expansions).cloned();
            // The first place it fits over nothing, else the middle.
            let free = |x: u8, y: u8| {
                board.as_ref().is_some_and(|b| nettai_battle::content::NaviCustRules::fits(b, &shape, x, y))
                    && cells(&shape, x, y).all(|(cx, cy)| at(content, &n, cx, cy).is_none())
            };
            let spot = (1..SIZE as u8 - 1).flat_map(|y| (1..SIZE as u8 - 1).map(move |x| (x, y))).find(|&(x, y)| free(x, y));
            let (x, y) = spot.unwrap_or((3, 3));
            parts.push(PlacedProgram { program, color, x, y, rotation: 0, compressed: false });
            state.selected = Some(parts.len() - 1);
        }
        Edit::Cell(x, y) => match at(content, &n, x as i32, y as i32) {
            Some(i) => {
                state.selected = Some(i);
                return false;
            }
            None => match state.selected.and_then(|i| parts.get_mut(i)) {
                Some(p) => (p.x, p.y) = (x, y),
                None => return false,
            },
        },
        Edit::Move(dx, dy) => {
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.x = (p.x as i8 + dx).clamp(0, SIZE as i8 - 1) as u8;
            p.y = (p.y as i8 + dy).clamp(0, SIZE as i8 - 1) as u8;
        }
        Edit::Rotate => {
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.rotation = (p.rotation + 1) % 4;
        }
        Edit::Compress(on) => {
            let Some(i) = state.selected else { return false };
            // A save compresses every copy of a program in one color.
            let (program, color) = (parts[i].program, parts[i].color);
            for p in parts.iter_mut().filter(|p| p.program == program && p.color == color) {
                p.compressed = on;
            }
        }
        Edit::Color(c) => {
            let Some(p) = state.selected.and_then(|i| parts.get_mut(i)) else { return false };
            p.color = c;
        }
        Edit::Remove => {
            let Some(i) = state.selected.take() else { return false };
            parts.remove(i);
        }
        Edit::UseGrid(_) | Edit::Search(_) => unreachable!(),
    }
    set(side, parts, expansions);
    true
}

/// The grid, drawn.
struct Grid {
    board: Option<nettai_battle::content::Board>,
    command_line: u8,
    /// Each program's cells, its color, whether it is a plus part and
    /// whether it is the picked one.
    parts: Vec<(Vec<(i32, i32)>, Color, bool, bool)>,
    side: usize,
}

impl canvas::Program<Msg> for Grid {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::from_rgb8(0x20, 0x28, 0x38));
        let at = |x: i32, y: i32| Point::new(x as f32 * CELL, y as f32 * CELL);
        let cell = Size::new(CELL - 2.0, CELL - 2.0);
        for y in 0..SIZE as i32 {
            for x in 0..SIZE as i32 {
                let kind = self.board.map_or(BoardCell::Off, |b| b[y as usize][x as usize]);
                let fill = match kind {
                    BoardCell::Off => continue,
                    BoardCell::Frame => Color::from_rgb8(0x38, 0x40, 0x58),
                    BoardCell::On if y == self.command_line as i32 => Color::from_rgb8(0x50, 0x68, 0x90),
                    BoardCell::On => Color::from_rgb8(0x5C, 0x60, 0x70),
                };
                frame.fill_rectangle(at(x, y) + iced::Vector::new(1.0, 1.0), cell, fill);
            }
        }
        for (cells, c, plus, picked) in &self.parts {
            for &(x, y) in cells {
                frame.fill_rectangle(at(x, y) + iced::Vector::new(4.0, 4.0), Size::new(CELL - 8.0, CELL - 8.0), *c);
                if *plus {
                    // A plus part's mark.
                    let mid = at(x, y) + iced::Vector::new(CELL / 2.0, CELL / 2.0);
                    let mark = Path::new(|p| {
                        p.move_to(mid + iced::Vector::new(-8.0, 0.0));
                        p.line_to(mid + iced::Vector::new(8.0, 0.0));
                        p.move_to(mid + iced::Vector::new(0.0, -8.0));
                        p.line_to(mid + iced::Vector::new(0.0, 8.0));
                    });
                    frame.stroke(&mark, Stroke::default().with_color(Color::WHITE).with_width(3.0));
                }
                if *picked {
                    let outline = Path::rectangle(at(x, y) + iced::Vector::new(2.0, 2.0), Size::new(CELL - 4.0, CELL - 4.0));
                    frame.stroke(&outline, Stroke::default().with_color(Color::WHITE).with_width(3.0));
                }
            }
        }
        vec![frame.into_geometry()]
    }

    fn update(&self, _: &mut (), event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Msg>> {
        if let iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            let p = cursor.position_in(bounds)?;
            let (x, y) = ((p.x / CELL) as u8, (p.y / CELL) as u8);
            if (x as usize) < SIZE && (y as usize) < SIZE {
                return Some(canvas::Action::publish(Msg::NaviCust(self.side, Edit::Cell(x, y))).and_capture());
            }
        }
        None
    }
}

pub fn view(e: &Editor, s: usize) -> Element<'_, Msg> {
    let c = &e.content;
    let side = e.side(s);
    let state = &e.navicust[s];
    let header = row![
        text(format!("{}: NaviCust", crate::view::SIDES[s])).size(20),
        space().width(Length::Fill),
        checkbox(side.navicust.is_some()).label("A grid of programs (else the stats set directly)").on_toggle(move |b| Msg::NaviCust(s, Edit::UseGrid(b))),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let Some(n) = side.navicust else {
        return column![header, crate::view::navicust_stats(e, s)].spacing(8).into();
    };
    let rules = nettai_match::navicust_rules(c, side);
    let sizes: Vec<Choice<u8>> = (0..rules.boards.len() as u8)
        .map(|x| {
            let b = &rules.boards[x as usize];
            let (w, h) = (b.iter().map(|r| r.iter().filter(|&&c| c == BoardCell::On).count()).max().unwrap_or(0), b.iter().filter(|r| r.contains(&BoardCell::On)).count());
            Choice { label: format!("{w}x{h} ({x} expansions)"), value: x }
        })
        .collect();
    let size = sizes.iter().find(|x| x.value == n.expansions).cloned();
    let parts: Vec<_> = n
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let def = c.navicust_program(p.program);
            let shape = def.placed_shape(p.compressed, p.rotation);
            let name = def.colors.get(p.color as usize).cloned().unwrap_or_default();
            (cells(&shape, p.x, p.y).collect(), color(&name), def.plus, state.selected == Some(i))
        })
        .collect();
    let grid = canvas_widget(Grid { board: rules.board(n.expansions).copied(), command_line: rules.command_line, parts, side: s })
        .width(Length::Fixed(CELL * SIZE as f32))
        .height(Length::Fixed(CELL * SIZE as f32));
    // The picked program.
    let picked: Element<Msg> = match state.selected.and_then(|i| n.iter().nth(i)) {
        Some(p) => {
            let def = c.navicust_program(p.program);
            let colors = def.colors.iter().enumerate().fold(row![].spacing(4), |r, (k, name)| {
                let b = button(text(name.as_str()).size(12)).on_press(Msg::NaviCust(s, Edit::Color(k as u8)));
                r.push(if k == p.color as usize { b.style(button::primary) } else { b.style(button::secondary) })
            });
            let mut col = column![
                text(format!("{} at ({}, {}), turned {}", e.names.navicust_program(c, p.program), p.x, p.y, p.rotation)).size(15),
                colors,
                row![
                    button("←").on_press(Msg::NaviCust(s, Edit::Move(-1, 0))),
                    button("↑").on_press(Msg::NaviCust(s, Edit::Move(0, -1))),
                    button("↓").on_press(Msg::NaviCust(s, Edit::Move(0, 1))),
                    button("→").on_press(Msg::NaviCust(s, Edit::Move(1, 0))),
                    button("Turn").on_press(Msg::NaviCust(s, Edit::Rotate)),
                    button("Remove").on_press(Msg::NaviCust(s, Edit::Remove)).style(button::danger),
                ]
                .spacing(4),
            ]
            .spacing(6);
            if def.compressed.is_some() {
                col = col.push(checkbox(p.compressed).label("Compressed").on_toggle(move |b| Msg::NaviCust(s, Edit::Compress(b))));
            }
            col.into()
        }
        None => text("Click a program on the grid to pick it; click an empty cell to move the picked one there.").size(13).into(),
    };
    // The programs to add, searched.
    let needle = state.search.to_lowercase();
    let mut programs: Vec<(String, NaviCustProgramHandle)> = (0..c.defs.navicust_programs.len() as u16)
        .map(NaviCustProgramHandle)
        .map(|h| (e.names.navicust_program(c, h), h))
        .filter(|(name, _)| needle.is_empty() || name.to_lowercase().contains(&needle))
        .collect();
    programs.sort();
    let list = programs.into_iter().fold(Column::new().spacing(2), |col, (name, h)| {
        let def = c.navicust_program(h);
        let adds = def.colors.iter().enumerate().fold(row![].spacing(3), |r, (k, cname)| {
            let swatch = button(text(" ").size(12)).style(move |_: &Theme, _| button::Style {
                background: Some(color(cname).into()),
                ..button::Style::default()
            });
            r.push(swatch.width(Length::Fixed(22.0)).on_press(Msg::NaviCust(s, Edit::Add(h, k as u8))))
        });
        col.push(
            row![
                text(name).size(14).width(Length::Fill),
                text(if def.plus { "plus" } else { "" }).size(12).color(Color::from_rgb(0.5, 0.5, 0.55)).width(Length::Fixed(34.0)),
                iced::widget::container(adds).width(Length::Fixed(80.0)),
                space().width(Length::Fixed(14.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
    });
    let left = column![
        row![text("Board").size(14), pick_list(sizes, size, move |x: Choice<u8>| Msg::NaviCust(s, Edit::Expansions(x.value)))].spacing(8).align_y(Alignment::Center),
        grid,
        text("The command line is the lit row. A color square adds a program in that color.").size(12),
        picked,
        crate::view::round_stats(e, s),
    ]
    .spacing(8)
    .width(Length::Fixed(CELL * SIZE as f32 + 120.0));
    let right = column![
        text_input("search programs", &state.search).on_input(move |t| Msg::NaviCust(s, Edit::Search(t))),
        scrollable(list).height(Length::Fill),
    ]
    .spacing(6)
    .width(Length::Fill);
    column![header, row![scrollable(left), right].spacing(16)].spacing(8).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pane's edits on BN6's content: a grid takes the stats back to
    /// what the save keeps; programs go where they fit, are picked, moved,
    /// turned, compressed by program and color, taken off; and the match
    /// stays one the checks accept.
    #[test]
    fn edits() {
        let content = nettai_match::testing::bn6_content();
        let mut m = nettai_match::draw::live(&content, 7, None).unwrap();
        let mut state = State::default();
        let side = &mut m.sides[0];
        assert!(update(&content, side, &mut state, Edit::UseGrid(true)));
        assert_eq!(side.navicust.unwrap().expansions, 2);
        assert!(side.stats_block(&content).keys().all(|k| nettai_match::stats::SAVE_FIELDS.contains(&k.as_str())));
        let program = |key: &str| content.defs.navicust_program_by_key(key).unwrap();
        update(&content, side, &mut state, Edit::Add(program("suprarmr"), 0));
        update(&content, side, &mut state, Edit::Add(program("hp-50"), 1));
        update(&content, side, &mut state, Edit::Add(program("hp-50"), 1));
        let n = side.navicust.unwrap();
        assert_eq!(n.len(), 3);
        assert_eq!(state.selected, Some(2));
        let problems = nettai_match::check::check_navicust(&content, side, &n);
        assert!(problems.is_empty(), "{problems:?}");
        // Compressing one HP+50 compresses the other (one program, one color).
        update(&content, side, &mut state, Edit::Compress(true));
        let n = side.navicust.unwrap();
        assert!(n.iter().filter(|p| p.program == program("hp-50")).all(|p| p.compressed));
        // Picked by a cell it covers; moved; turned; taken off.
        let first = n.iter().next().unwrap();
        update(&content, side, &mut state, Edit::Cell(first.x, first.y));
        assert_eq!(state.selected, Some(0));
        update(&content, side, &mut state, Edit::Move(0, 1));
        update(&content, side, &mut state, Edit::Rotate);
        let moved = side.navicust.unwrap().iter().next().unwrap();
        assert_eq!((moved.y, moved.rotation), (first.y + 1, 1));
        update(&content, side, &mut state, Edit::Remove);
        assert_eq!(side.navicust.unwrap().len(), 2);
        let problems = nettai_match::check_match(&content, &m);
        assert!(problems.is_empty(), "{problems:?}");
        let side = &mut m.sides[0];
        assert!(update(&content, side, &mut state, Edit::UseGrid(false)));
        assert!(side.navicust.is_none());
    }
}
