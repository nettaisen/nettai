//! nettai-editor: edit a match file (docs/frontend.md §6) and play it with
//! nettai-frontend. README.md, "The match editor".

mod app;
mod names;
mod pictures;
mod view;

use app::{Editor, Options, Tab};
use names::Lang;
use std::path::PathBuf;
use std::sync::Arc;

const USAGE: &str = "\
usage: nettai-editor [OPTIONS] [MATCH.toml]

  --content DIR    the battle content directory (default: $NETTAI_CONTENT,
                   else this repository's content/)
  --pack DIR       the content pack, for the chips' pictures (default:
                   $BN6_PACK, else data/content/bn6)
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
        content_root: nettai_content::root::content(),
        pack: std::env::var_os("BN6_PACK").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("data/content/bn6")),
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
            "--content" => o.content_root = value("--content")?.into(),
            "--pack" => o.pack = value("--pack")?.into(),
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
const FONT: &[u8] = include_bytes!("../../nettai-frontend/fonts/murecho/Murecho-VariableFont_wght.ttf");

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
    let content = match nettai_content::pack::load_battle(&options.content_root, &options.pack) {
        Ok((c, _)) => Arc::new(c),
        Err(r) => {
            for i in &r.issues {
                eprintln!("{i}");
            }
            fail(format!(
                "can't load the content {} with the pack {} (--content, --pack)",
                options.content_root.display(),
                options.pack.display()
            ))
        }
    };
    let pictures = pictures::Pictures::load(&options.pack).unwrap_or_else(|e| {
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
