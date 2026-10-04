//! nettai-editor: edit a match file (docs/frontend.md §6) and play it with
//! nettai-frontend. README.md, "The match editor".

mod app;
mod levels;
mod names;
mod navicust;
mod pictures;
mod view;

use app::{Editor, Options, Tab};
use names::Lang;
use std::sync::Arc;

const USAGE: &str = "\
usage: nettai-editor [OPTIONS] [MATCH.toml]

  The content packs (the chips' pictures) and the content's folders are
  found as nettai-frontend finds them: every pack in the packs directory,
  $NETTAI_PACKS, else data/content, each by its game; every folder of the
  content directory whose game's pack is found and that loads.
  --content DIR    the battle content directory, its folders one namespace
                   (default: $NETTAI_CONTENT, else this repository's
                   content/)
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

/// Show what loading found (warnings and errors).
fn show(report: &nettai_content::report::Report) {
    for i in report.issues.iter().filter(|i| i.level != nettai_content::report::Level::Note) {
        eprintln!("{i}");
    }
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
    // Every pack found, and the game the editor loads with its pack (as the
    // frontend does: nettai_content::pack::load_game).
    let mut report = nettai_content::report::Report::default();
    let found = nettai_content::pack::find(&nettai_content::pack::packs_dir(), &options.packs, &mut report);
    show(&report);
    let found = found.unwrap_or_else(|| fail("can't read the packs given (--pack, $BN6_PACK)"));
    // (One game a match, docs/design/content-model-v2.md §4.0: the
    // editor's is BN6 until it chooses one.)
    let loaded = nettai_content::pack::load_game(options.content.as_deref(), nettai_match::DEFAULT_GAME, &found).unwrap_or_else(|r| {
        show(&r);
        fail("can't load the battle content (--content, --pack)")
    });
    show(&loaded.report);
    (options.content_dir, options.games) = (loaded.dir.clone(), vec![loaded.game.clone()]);
    let content = Arc::new(loaded.content);
    let pictures = pictures::Pictures::load(&content, std::slice::from_ref(&loaded.pack)).unwrap_or_else(|e| {
        eprintln!("{e}: the chips have no pictures");
        pictures::Pictures::default()
    });
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
