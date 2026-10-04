//! nettai-editor: edit a match file (docs/frontend.md §6) and play it with
//! nettai-frontend. README.md, "The match editor".

mod app;
mod levels;
mod load;
mod names;
mod navicust;
mod pictures;
mod view;

use app::{Editor, Options, Tab};
use names::Lang;

const USAGE: &str = "\
usage: nettai-editor [OPTIONS] [MATCH.toml]

  A match is of one game, BN6 or BN5, picked first in the arena pane: the
  file's, else BN6. Its content and its pack (the chips' pictures) are
  found as nettai-frontend finds them: the packs in the packs directory,
  $NETTAI_PACKS, else data/content, each by its game.
  --content DIR    the battle content directory (default: $NETTAI_CONTENT,
                   else this repository's content/)
  --pack DIR       a pack's directory, in place of the found pack of its game
                   (again for another game's), handed to the frontend too
  --lang LANG      names in en (default) or ja
  --frontend PATH  the nettai-frontend program Play runs (default: the one
                   beside this program, else nettai-frontend on the PATH)
  --tab NAME       start on a pane: arena, or left- or right- and navi,
                   folder, crosses, cards, navicust, stats
  --screenshot PNG write the window to PNG once it has drawn, and quit";

fn fail(msg: impl std::fmt::Display) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

fn parse() -> Result<Options, String> {
    let mut o = Options {
        content: None,
        content_dir: std::path::PathBuf::new(),
        games: Vec::new(),
        packs: Vec::new(),
        frontend: None,
        file: None,
        lang: Lang::En,
        tab: Tab::Arena,
        screenshot: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--content" => o.content = Some(value("--content")?.into()),
            "--pack" => o.packs.push(value("--pack")?.into()),
            "--lang" => {
                let l = value("--lang")?;
                o.lang = Lang::from_code(&l).ok_or_else(|| format!("no language {l:?} (en or ja)"))?;
            }
            "--frontend" => o.frontend = Some(value("--frontend")?.into()),
            "--tab" => {
                let t = value("--tab")?;
                o.tab = Tab::from_name(&t).ok_or_else(|| format!("no pane {t:?}"))?;
            }
            "--screenshot" => o.screenshot = Some(value("--screenshot")?.into()),
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => o.file = Some(s.into()),
        }
    }
    Ok(o)
}

/// The font the editor writes with: the frontend's bundled Murecho (Latin,
/// kana and kanji, for the Japanese names).
const FONT: &[u8] = nettai_render::vfont::BUNDLED;

fn main() -> iced::Result {
    let options = match parse() {
        Ok(o) => o,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    let mut options = options;
    // The game's content, as the frontend loads it: the file's game, else
    // BN6.
    let game = match &options.file {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|t| nettai_match::file::game_of(&t))
            .unwrap_or_else(|e| fail(format!("{}: {e}", path.display()))),
        None => nettai_match::DEFAULT_GAME.to_string(),
    };
    let loaded = load::load_game(options.content.as_deref(), &options.packs, &game).unwrap_or_else(|e| fail(e));
    (options.content_dir, options.games) = (loaded.dir, vec![loaded.game]);
    let (content, pictures) = (loaded.content, loaded.pictures);
    // A round that doesn't start is a problem the editor shows, not a
    // message on the terminal.
    std::panic::set_hook(Box::new(|_| {}));
    let shot = options.screenshot.is_some();
    let boot = std::cell::RefCell::new(Some((content, pictures, options)));
    let start = move || {
        let (content, pictures, options) = boot.borrow_mut().take().expect("the editor boots once");
        Editor::new(content, pictures, options)
    };
    iced::application(start, Editor::update, view::view)
        .title(Editor::title)
        .theme(view::theme)
        .subscription(move |_| if shot { iced::window::frames().map(|_| app::Msg::Frame) } else { iced::Subscription::none() })
        .font(FONT)
        .default_font(iced::Font::with_name("Murecho"))
        .settings(iced::Settings { default_text_size: iced::Pixels(15.0), ..iced::Settings::default() })
        .window_size(iced::Size::new(1180.0, 800.0))
        .run()
}
