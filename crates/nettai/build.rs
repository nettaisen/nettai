//! The app's UI, compiled: `ui/app.slint`, with its translations bundled.
//!
//! The translations are gettext catalogs, one a language:
//! `lang/<language>/LC_MESSAGES/nettai.po`, each written from the template
//! `lang/nettai.pot` (what `slint-tr-extractor` takes from the .slint
//! files; docs/app.md, "Languages"). Every catalog there is bundled, so
//! adding a language is adding its file. To them this adds a pseudo-locale,
//! `pseudo`, made from the template: each string lengthened and its letters
//! accented, to show which layouts can't take a longer text and which
//! strings don't go through the catalog.
//!
//! It also writes `languages.rs`: the languages bundled, each by its code
//! and its own name (the English one, the source, first).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets it"));
    let lang = manifest.join("lang");
    println!("cargo:rerun-if-changed={}", lang.display());

    // The catalogs, and the pseudo-locale, in one tree for Slint to bundle.
    let bundle = out.join("lang");
    let _ = std::fs::remove_dir_all(&bundle);
    std::fs::create_dir_all(&bundle).expect("the build's folder is writable");
    let mut languages = vec![("en".to_string(), "English".to_string())];
    let mut codes: Vec<PathBuf> = std::fs::read_dir(&lang)
        .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.join("LC_MESSAGES/nettai.po").is_file()).collect())
        .unwrap_or_default();
    codes.sort();
    for dir in codes {
        let code = dir.file_name().and_then(|n| n.to_str()).expect("a language's folder is named by its code").to_string();
        let po = dir.join("LC_MESSAGES/nettai.po");
        println!("cargo:rerun-if-changed={}", po.display());
        let text = std::fs::read_to_string(&po).unwrap_or_else(|e| panic!("can't read {}: {e}", po.display()));
        let name = own_name(&text).unwrap_or_else(|| code.clone());
        write(&bundle.join(&code).join("LC_MESSAGES/nettai.po"), &text);
        languages.push((code, name));
    }
    let pot = lang.join("nettai.pot");
    println!("cargo:rerun-if-changed={}", pot.display());
    if let Ok(template) = std::fs::read_to_string(&pot) {
        write(&bundle.join("pseudo/LC_MESSAGES/nettai.po"), &pseudo_catalog(&template));
        languages.push(("pseudo".to_string(), pseudo("Pseudo")));
    }
    let mut list = String::from("/// The languages bundled: each one's code and its own name.\npub const LANGUAGES: &[(&str, &str)] = &[\n");
    for (code, name) in &languages {
        let _ = writeln!(list, "    ({code:?}, {name:?}),");
    }
    list.push_str("];\n");
    write(&out.join("languages.rs"), &list);

    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations(bundle)
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/app.slint", config).expect("the UI compiles");
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a file in a folder")).expect("the build's folder is writable");
    std::fs::write(path, text).expect("the build's folder is writable");
}

/// A catalog's name for its language, in it: the translation of the
/// language's English name, which the UI asks for in the context
/// "language" (`@tr("language" => "English")`).
fn own_name(po: &str) -> Option<String> {
    let entries = entries(po);
    entries.into_iter().find(|e| e.context.as_deref() == Some("language") && e.id == "English").and_then(|e| e.strs.into_iter().next()).filter(|s| !s.is_empty())
}

/// A catalog entry: its context, its string and plural, and its
/// translations.
#[derive(Default)]
struct Entry {
    context: Option<String>,
    id: String,
    plural: Option<String>,
    strs: Vec<String>,
}

/// A .po or .pot file's entries (gettext's format: keywords followed by
/// quoted strings, continued on the next lines; comments skipped).
fn entries(po: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut entry = Entry::default();
    // (The field the quoted lines go to: 0 context, 1 id, 2 plural, 3+ the
    // translations.)
    let mut field = None;
    let push = |entry: &mut Entry, out: &mut Vec<Entry>| {
        if !entry.id.is_empty() || !entry.strs.is_empty() {
            out.push(std::mem::take(entry));
        }
    };
    for line in po.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (keyword, rest) = match line.find(' ') {
            Some(at) if !line.starts_with('"') => (&line[..at], line[at + 1..].trim()),
            _ => ("", line),
        };
        let text = unquote(rest);
        match keyword {
            "msgctxt" => {
                push(&mut entry, &mut out);
                entry.context = Some(text);
                field = Some(0);
            }
            "msgid" => {
                if entry.context.is_none() || !entry.id.is_empty() {
                    push(&mut entry, &mut out);
                }
                entry.id = text;
                field = Some(1);
            }
            "msgid_plural" => {
                entry.plural = Some(text);
                field = Some(2);
            }
            k if k.starts_with("msgstr") => {
                entry.strs.push(text);
                field = Some(3 + entry.strs.len() - 1);
            }
            "" => match field {
                Some(0) => entry.context.get_or_insert_default().push_str(&text),
                Some(1) => entry.id.push_str(&text),
                Some(2) => entry.plural.get_or_insert_default().push_str(&text),
                Some(n) => entry.strs[n - 3].push_str(&text),
                None => {}
            },
            _ => {}
        }
    }
    push(&mut entry, &mut out);
    out
}

fn unquote(s: &str) -> String {
    let s = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s);
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(c) => out.push(c),
            None => {}
        }
    }
    out
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The pseudo-locale's catalog: every string of the template, pseudo.
fn pseudo_catalog(template: &str) -> String {
    let mut out = String::from(
        "msgid \"\"\nmsgstr \"\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Language: pseudo\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
    );
    for e in entries(template).into_iter().filter(|e| !e.id.is_empty()) {
        if let Some(c) = &e.context {
            let _ = writeln!(out, "msgctxt {}", quote(c));
        }
        let _ = writeln!(out, "msgid {}", quote(&e.id));
        match &e.plural {
            Some(p) => {
                let _ = writeln!(out, "msgid_plural {}", quote(p));
                let _ = writeln!(out, "msgstr[0] {}", quote(&pseudo(&e.id)));
                let _ = writeln!(out, "msgstr[1] {}", quote(&pseudo(p)));
            }
            None => {
                let _ = writeln!(out, "msgstr {}", quote(&pseudo(&e.id)));
            }
        }
        out.push('\n');
    }
    out
}

/// A string made longer (each vowel doubled, and brackets) and accented,
/// its placeholders (`{}`, `{0}`, `{n}`) kept as they are.
fn pseudo(s: &str) -> String {
    let mut out = String::from("[");
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            out.push(c);
            for d in chars.by_ref() {
                out.push(d);
                if d == '}' {
                    break;
                }
            }
            continue;
        }
        let accented = match c {
            'a' => "àá",
            'e' => "èé",
            'i' => "ìí",
            'o' => "òó",
            'u' => "ùú",
            'y' => "ýÿ",
            'A' => "ÀÁ",
            'E' => "ÈÉ",
            'I' => "ÌÍ",
            'O' => "ÒÓ",
            'U' => "ÙÚ",
            'c' => "ç",
            'n' => "ñ",
            's' => "š",
            'z' => "ž",
            'C' => "Ç",
            'N' => "Ñ",
            'S' => "Š",
            'Z' => "Ž",
            'D' => "Ð",
            'g' => "ğ",
            'G' => "Ğ",
            'l' => "ł",
            'L' => "Ł",
            'r' => "ř",
            'R' => "Ř",
            't' => "ţ",
            'T' => "Ţ",
            _ => "",
        };
        if accented.is_empty() {
            out.push(c);
        } else {
            out.push_str(accented);
        }
    }
    out.push(']');
    out
}
