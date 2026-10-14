//! Settings (docs/design/app-ui.md §6): General, Video, Audio, Online, Controls, About, on one scrolling page.
//! Every change is applied and kept at once (`crate::settings`). The About group's SHOW LICENSES opens About in
//! the page's place (`about`), and Back there comes back to the page.

mod about;

use crate::input::{Action, Bindings, KeyPress, key_cap, key_name, pad_cap, pad_name};
use crate::screens::{Screen, ScreenId};
use crate::settings::{MAX_INPUT_DELAY, clean_name};
use crate::shell::Shell;
use crate::sound::Cue;
use gamepad_facade::Button;
use nettai_ui::{PadBinding, PadButton, SettingsView};
use slint::winit_030::winit::keyboard::KeyCode;
use slint::{ComponentHandle, ModelRc, VecModel};

/// Where a font's or a package's row is in About's list (the screenshot pass reads Slint's license).
#[cfg(any(test, feature = "tour"))]
pub(crate) use about::place_of;

/// Each group's first control, and the room over it for what the group has above it (its title; in Controls the
/// columns' heads, in About the version's row): where the list at the left jumps to.
const GROUPS: [(&str, f32); 6] = [("set.name", 40.0), ("set.picture", 40.0), ("set.music", 40.0), ("set.delay", 40.0), ("set.key", 68.0), ("set.folder", 92.0)];

/// The picture sizes offered besides the largest that fits: 1 to 8 times 240 by 160.
const SIZES: u32 = 8;

/// The group a control of the page is in.
fn group_of(id: &str) -> Option<i32> {
    Some(match id {
        "set.name" | "set.language" => 0,
        "set.picture" | "set.fullscreen" | "set.motion" | "set.graphics" => 1,
        "set.music" | "set.effects" => 2,
        "set.delay" => 3,
        "set.key" | "set.button" | "set.reset" => 4,
        "set.folder" | "set.about" => 5,
        _ => return None,
    })
}

#[derive(Default)]
pub struct Settings {
    /// The button whose key (false) or pad button (true) is being asked for.
    asking: Option<(usize, bool)>,
    /// About, shown in the page's place while it is open.
    about: about::About,
}

impl Settings {
    fn size_label(size: u32) -> String {
        format!("{size}× ({} × {})", 240 * size, 160 * size)
    }

    /// The bindings, as the controls' boxes show them.
    fn show_bindings(&self, shell: &Shell) {
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        let keys: Vec<slint::SharedString> = shell.bindings.keys.iter().map(|k| key_cap(*k).into()).collect();
        let buttons: Vec<PadBinding> = shell
            .bindings
            .pad
            .iter()
            .map(|b| {
                let face = match b {
                    Button::South => PadButton::South,
                    Button::East => PadButton::East,
                    Button::West => PadButton::West,
                    Button::North => PadButton::North,
                    _ => PadButton::None,
                };
                PadBinding { face, text: pad_cap(*b).into() }
            })
            .collect();
        view.set_keys(ModelRc::new(VecModel::from(keys)));
        view.set_buttons(ModelRc::new(VecModel::from(buttons)));
        view.set_asking_key(self.asking.filter(|a| !a.1).map_or(-1, |a| a.0 as i32));
        view.set_asking_button(self.asking.filter(|a| a.1).map_or(-1, |a| a.0 as i32));
    }

    fn set_language(&mut self, shell: &mut Shell, code: &'static str) {
        shell.settings.language = Some(code.to_string());
        shell.save_settings();
        shell.set_language(code);
    }

    fn set_picture_size(&mut self, shell: &mut Shell, size: u32) {
        shell.settings.picture_size = size.min(SIZES);
        shell.save_settings();
        // (The picture on screen is fitted again at the next frame.)
        if let Some(b) = &mut shell.battle {
            b.stage.surface.stale = true;
        }
        self.show(shell);
    }

    fn set_full_screen(&mut self, shell: &mut Shell, on: bool) {
        shell.settings.full_screen = on;
        shell.save_settings();
        shell.ui().window().set_fullscreen(on);
        self.show(shell);
    }

    fn set_reduce_motion(&mut self, shell: &mut Shell, reduce: bool) {
        shell.set_reduce_motion(reduce);
        self.show(shell);
    }

    fn set_enhanced(&mut self, shell: &mut Shell, enhanced: bool) {
        shell.settings.enhanced = enhanced;
        shell.save_settings();
        self.show(shell);
    }

    fn set_levels(&mut self, shell: &mut Shell, music: u32, effects: u32) {
        (shell.settings.music, shell.settings.effects) = (music.min(100), effects.min(100));
        shell.set_sound_levels(shell.settings.music, shell.settings.effects);
        shell.save_settings();
        self.show(shell);
    }
}

impl Screen for Settings {
    fn main(&self) -> &'static str {
        "set.name"
    }

    fn show(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        let s = &shell.settings;
        view.set_name(s.name.as_str().into());
        view.set_language(crate::lang::name(shell.lang).into());
        view.set_picture_size(if s.picture_size == 0 { Default::default() } else { Settings::size_label(s.picture_size).into() });
        view.set_full_screen(s.full_screen);
        view.set_reduce_motion(shell.reduces_motion());
        view.set_enhanced(s.enhanced);
        view.set_music(s.music as i32);
        view.set_effects(s.effects as i32);
        view.set_input_delay(s.input_delay as i32);
        view.set_data_folder(crate::paths::data().display().to_string().into());
        self.show_bindings(shell);
        if self.about.is_open() {
            self.about.show(shell);
        }
    }

    fn hide(&mut self, shell: &mut Shell) {
        self.asking = None;
        // (Shown again, Settings show their page, the focus on SHOW LICENSES.)
        if self.about.is_open() {
            self.about.close(shell);
        }
    }

    fn activate(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if self.about.is_open() {
            return self.about.activate(shell, id, index);
        }
        match id {
            "group" => shell.nav.focus(GROUPS[index.clamp(0, 5) as usize].0, 0),
            "set.language" => {
                let offered = crate::lang::offered(shell.lang);
                let options: Vec<String> = offered.iter().map(|(_, name)| name.to_string()).collect();
                let chosen = offered.iter().position(|(code, _)| *code == shell.lang).unwrap_or(0);
                shell.pick(id, 0, options, chosen);
            }
            "set.picture" => {
                let largest = shell.ui().global::<SettingsView>().get_largest().to_string();
                let options = std::iter::once(largest).chain((1..=SIZES).map(Settings::size_label)).collect();
                shell.pick(id, 0, options, shell.settings.picture_size as usize);
            }
            "set.fullscreen" => self.set_full_screen(shell, !shell.settings.full_screen),
            "set.motion" => self.set_reduce_motion(shell, !shell.reduces_motion()),
            "set.graphics" => self.set_enhanced(shell, !shell.settings.enhanced),
            "set.key" | "set.button" => {
                self.asking = Some((index.clamp(0, 9) as usize, id == "set.button"));
                self.show_bindings(shell);
            }
            "set.reset" => {
                shell.set_bindings(Bindings::default());
                self.show_bindings(shell);
            }
            "set.folder" => show_folder(&crate::paths::data()),
            "set.about" => self.about.open(shell),
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, id: &str, _index: i32, by: i32) {
        match id {
            "set.language" => self.set_language(shell, crate::lang::step(shell.lang, by)),
            "set.picture" => self.set_picture_size(shell, (shell.settings.picture_size as i32 + by).rem_euclid(SIZES as i32 + 1) as u32),
            "set.fullscreen" => self.set_full_screen(shell, by > 0),
            "set.motion" => self.set_reduce_motion(shell, by > 0),
            // (Enhanced is the first of the two.)
            "set.graphics" => self.set_enhanced(shell, by < 0),
            "set.music" => self.set_levels(shell, (shell.settings.music as i32 + 5 * by).clamp(0, 100) as u32, shell.settings.effects),
            "set.effects" => {
                self.set_levels(shell, shell.settings.music, (shell.settings.effects as i32 + 5 * by).clamp(0, 100) as u32);
                shell.sound.play(Cue::Cursor);
            }
            "set.delay" => {
                shell.settings.input_delay = (shell.settings.input_delay as i32 + by).clamp(0, MAX_INPUT_DELAY as i32) as u32;
                shell.save_settings();
                self.show(shell);
            }
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, id: &str, _index: i32, choice: i32) {
        match id {
            "set.language" => {
                if let Some((code, _)) = crate::lang::offered(shell.lang).get(choice.max(0) as usize) {
                    self.set_language(shell, *code);
                }
            }
            "set.picture" => self.set_picture_size(shell, choice.max(0) as u32),
            "set.fullscreen" => self.set_full_screen(shell, choice == 1),
            "set.motion" => self.set_reduce_motion(shell, choice == 1),
            "set.graphics" => self.set_enhanced(shell, choice == 0),
            "set.music" => self.set_levels(shell, choice.clamp(0, 100) as u32, shell.settings.effects),
            "set.effects" => self.set_levels(shell, shell.settings.music, choice.clamp(0, 100) as u32),
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, id: &str, _index: i32, text: &str, done: bool) {
        if id != "set.name" {
            return;
        }
        let name = clean_name(text);
        let ui = shell.ui();
        ui.global::<SettingsView>().set_name(name.as_str().into());
        if done {
            shell.set_name(&name);
        }
    }

    fn focused(&mut self, shell: &mut Shell, id: &str, index: i32) {
        if self.about.is_open() {
            return self.about.focused(shell, id, index);
        }
        let ui = shell.ui();
        let view = ui.global::<SettingsView>();
        if id == "group" {
            // The list's row under the cursor: its group comes to the page's top.
            view.set_group(index);
            let (first, room) = GROUPS[index.clamp(0, 5) as usize];
            shell.nav.reveal_at_top(&ui, first, 0, room);
        } else if let Some(group) = group_of(id) {
            view.set_group(group);
        }
        if self.asking.take().is_some() {
            self.show_bindings(shell);
        }
    }

    fn key(&mut self, shell: &mut Shell, key: &KeyPress) -> bool {
        let Some((button, pad)) = self.asking else { return false };
        if key.repeat {
            return true;
        }
        if key.code == KeyCode::Escape || pad {
            self.asking = None;
        } else if key_name(key.code).is_some() {
            let mut bindings = shell.bindings.clone();
            bindings.bind_key(button, key.code);
            shell.set_bindings(bindings);
            self.asking = None;
            shell.sound.play(Cue::Pick);
        } else {
            // (A key that can't be bound: the question stands.)
            shell.sound.play(Cue::Refused);
        }
        self.show_bindings(shell);
        true
    }

    fn scroll(&mut self, shell: &mut Shell, action: Action) -> bool {
        self.about.is_open() && self.about.scroll(shell, action)
    }

    /// Back in About: within it, else to the settings, the focus back on SHOW LICENSES.
    fn back(&mut self, shell: &mut Shell) -> bool {
        if !self.about.is_open() {
            return false;
        }
        if !self.about.back(shell) {
            self.about.close(shell);
            shell.nav.enter(ScreenId::Settings.name(), self.main());
        }
        true
    }

    fn button(&mut self, shell: &mut Shell, button: Button) -> bool {
        let Some((at, pad)) = self.asking else { return false };
        if pad && pad_name(button).is_some() {
            let mut bindings = shell.bindings.clone();
            bindings.bind_pad(at, button);
            shell.set_bindings(bindings);
            shell.sound.play(Cue::Pick);
        }
        self.asking = None;
        self.show_bindings(shell);
        true
    }
}

/// Show a folder in the system's file manager.
fn show_folder(path: &std::path::Path) {
    let _ = std::fs::create_dir_all(path);
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(why) = std::process::Command::new(program).arg(path).spawn() {
        crate::errors::log("showing the data folder", &why);
    }
}
