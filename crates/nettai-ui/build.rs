//! The app's UI, compiled: `ui/app.slint`, with its translations bundled.
//!
//! Every string of the UI goes through Slint's `@tr`, written in English. A language is a folder of gettext
//! catalogs, `lang/<code>/*.po`: one file for each part of the app (a screen's strings in its own file, so two
//! screens' translators don't meet in one), which this merges into the one catalog Slint bundles. Adding a
//! language is adding its folder.
//!
//! To them this adds a pseudo-locale, `pseudo`: every `@tr` string found in the .slint files, lengthened and
//! accented. A layout that can't take a longer text shows it, and so does a string that doesn't go through
//! `@tr`. It is made from the sources, not from a template, so no string is missing from it.
//!
//! It also writes `languages.rs` (the languages bundled, each by its code and its own name, English, the
//! source, first), and says which strings a language has no translation for, and which of its translations
//! no string asks for any more.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// A string to translate: its context (`@tr("context" => "text")`) and its text.
type Key = (Option<String>, String);

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets it"));
    let domain = std::env::var("CARGO_PKG_NAME").expect("cargo sets it");
    let lang = manifest.join("lang");
    println!("cargo:rerun-if-changed={}", lang.display());

    // Every string the UI asks for, from its sources.
    let mut sources = Vec::new();
    slint_files(&manifest.join("ui"), &mut sources);
    sources.sort();
    let mut strings: BTreeMap<Key, Option<String>> = BTreeMap::new();
    for path in &sources {
        println!("cargo:rerun-if-changed={}", path.display());
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()));
        for (key, plural) in tr_strings(&text, path) {
            strings.entry(key).or_insert(plural);
        }
    }

    // The catalogs, and the pseudo-locale, in one tree for Slint to bundle. (Slint has cargo run this script
    // again when the tree changes, so a file is written only when what it says changed, and the tree is never
    // made anew: else every build would find it changed, and compile the UI again.)
    let bundle = out.join("lang");
    std::fs::create_dir_all(&bundle).expect("the build's folder is writable");
    let mut languages = vec![("en".to_string(), "English".to_string())];
    let mut folders: Vec<PathBuf> = std::fs::read_dir(&lang).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    folders.sort();
    for folder in folders {
        let code = folder.file_name().and_then(|n| n.to_str()).expect("a language's folder is named by its code").to_string();
        let mut files: Vec<PathBuf> =
            std::fs::read_dir(&folder).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "po")).collect()).unwrap_or_default();
        files.sort();
        if files.is_empty() {
            continue;
        }
        let (mut header, mut merged) = (None, BTreeMap::<Key, Entry>::new());
        for file in &files {
            println!("cargo:rerun-if-changed={}", file.display());
            let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("can't read {}: {e}", file.display()));
            for e in entries(&text) {
                if e.id.is_empty() {
                    header.get_or_insert(e.strs.concat());
                    continue;
                }
                let key = (e.context.clone(), e.id.clone());
                match merged.get(&key) {
                    Some(old) if old.strs != e.strs => {
                        println!("cargo:warning={code}: {:?} is translated twice, differently (the second is in {})", e.id, file.display());
                    }
                    Some(_) => {}
                    None => {
                        merged.insert(key, e);
                    }
                }
            }
        }
        let own = merged.get(&(Some("language".to_string()), "English".to_string())).and_then(|e| e.strs.first().cloned()).filter(|s| !s.is_empty());
        // What the language lacks, and what it has that nothing asks for.
        let missing: Vec<&str> = strings.keys().filter(|k| merged.get(*k).is_none_or(|e| e.strs.iter().all(String::is_empty))).map(|k| k.1.as_str()).collect();
        if !missing.is_empty() {
            println!("cargo:warning={code}: {} strings have no translation (lang/{code}/): {}", missing.len(), list(&missing));
        }
        let stale: Vec<&str> =
            merged.keys().filter(|k| !strings.contains_key(*k) && k.0.as_deref() != Some("language")).map(|k| k.1.as_str()).collect();
        if !stale.is_empty() {
            println!("cargo:warning={code}: {} translations are of strings the UI no longer has (lang/{code}/): {}", stale.len(), list(&stale));
        }
        let header = header.unwrap_or_else(|| format!("Content-Type: text/plain; charset=UTF-8\nLanguage: {code}\n"));
        write(&bundle.join(&code).join("LC_MESSAGES").join(format!("{domain}.po")), &catalog(&header, merged.values()));
        languages.push((code.clone(), own.unwrap_or(code)));
    }
    let pseudo_entries: Vec<Entry> = strings
        .iter()
        .map(|((context, id), plural)| Entry {
            context: context.clone(),
            id: id.clone(),
            plural: plural.clone(),
            strs: std::iter::once(pseudo(id)).chain(plural.as_deref().map(pseudo)).collect(),
        })
        .collect();
    let header = "Content-Type: text/plain; charset=UTF-8\nLanguage: pseudo\nPlural-Forms: nplurals=2; plural=(n != 1);\n";
    write(&bundle.join("pseudo/LC_MESSAGES").join(format!("{domain}.po")), &catalog(header, pseudo_entries.iter()));
    languages.push(("pseudo".to_string(), pseudo("Pseudo")));
    // (A language whose folder is gone leaves the tree.)
    for entry in std::fs::read_dir(&bundle).into_iter().flatten().flatten() {
        if !languages.iter().any(|(code, _)| entry.file_name().to_str() == Some(code)) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }

    let mut code = String::from("/// The languages bundled: each one's code and its own name.\npub const LANGUAGES: &[(&str, &str)] = &[\n");
    for (c, name) in &languages {
        let _ = writeln!(code, "    ({c:?}, {name:?}),");
    }
    code.push_str("];\n");
    write(&out.join("languages.rs"), &code);

    let config = slint_build::CompilerConfiguration::new()
        .with_bundled_translations(bundle)
        .with_default_translation_context(slint_build::DefaultTranslationContext::None);
    slint_build::compile_with_config("ui/app.slint", config).expect("the UI compiles");
}

/// A few of `strings`, for a warning's line.
fn list(strings: &[&str]) -> String {
    let shown: Vec<String> = strings.iter().take(6).map(|s| format!("{s:?}")).collect();
    format!("{}{}", shown.join(", "), if strings.len() > shown.len() { ", ..." } else { "" })
}

/// Write `text` to `path`, unless it says that already (its time of change is what cargo goes by).
fn write(path: &Path, text: &str) {
    if std::fs::read_to_string(path).is_ok_and(|was| was == text) {
        return;
    }
    std::fs::create_dir_all(path.parent().expect("a file in a folder")).expect("the build's folder is writable");
    std::fs::write(path, text).expect("the build's folder is writable");
}

/// The .slint files under `dir`.
fn slint_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("can't list {}: {e}", dir.display())).flatten() {
        let path = entry.path();
        if path.is_dir() {
            slint_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "slint") {
            out.push(path);
        }
    }
}

/// The strings `source` (a .slint file's text) asks to have translated: each `@tr("text")`,
/// `@tr("context" => "text")` and `@tr("one" | "many" % n)` outside a comment, as its key and its plural.
fn tr_strings(source: &str, path: &Path) -> Vec<(Key, Option<String>)> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            // A comment: to the line's end, or to its close.
            b'/' if bytes.get(at + 1) == Some(&b'/') => at = source[at..].find('\n').map_or(bytes.len(), |n| at + n),
            b'/' if bytes.get(at + 1) == Some(&b'*') => at = source[at + 2..].find("*/").map_or(bytes.len(), |n| at + n + 4),
            // A string that isn't a `@tr`'s: passed over, so what it holds isn't read as code.
            b'"' => at = string(source, at, path).1,
            b'@' if source[at..].starts_with("@tr(") => {
                let (first, mut next) = string(source, skip_space(source, at + 4), path);
                let (mut context, mut id) = (None, first);
                next = skip_space(source, next);
                if source[next..].starts_with("=>") {
                    let (text, after) = string(source, skip_space(source, next + 2), path);
                    (context, id) = (Some(id), text);
                    next = skip_space(source, after);
                }
                let mut plural = None;
                if source[next..].starts_with('|') {
                    let (text, after) = string(source, skip_space(source, next + 1), path);
                    plural = Some(text);
                    next = after;
                }
                out.push(((context, id), plural));
                at = next;
            }
            _ => at += 1,
        }
    }
    out
}

fn skip_space(source: &str, at: usize) -> usize {
    at + source[at..].len() - source[at..].trim_start().len()
}

/// The string literal that starts at `at` in a .slint file, unescaped, and where it ends.
fn string(source: &str, at: usize, path: &Path) -> (String, usize) {
    let line = || source[..at].matches('\n').count() + 1;
    assert!(source[at..].starts_with('"'), "{}:{}: a string is expected here (a `@tr` of something else can't be translated)", path.display(), line());
    let mut out = String::new();
    let mut chars = source[at + 1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (out, at + 1 + i + 1),
            '\\' => match chars.next() {
                Some((_, 'n')) => out.push('\n'),
                Some((_, 'u')) => {
                    // `\u{1F600}`.
                    let digits: String = chars.by_ref().map(|(_, d)| d).skip_while(|d| *d == '{').take_while(|d| *d != '}').collect();
                    out.extend(u32::from_str_radix(&digits, 16).ok().and_then(char::from_u32));
                }
                Some((_, other)) => out.push(other),
                None => break,
            },
            c => out.push(c),
        }
    }
    panic!("{}:{}: a string doesn't end", path.display(), line());
}

/// A catalog entry: its context, its string and plural, and its translations.
#[derive(Default)]
struct Entry {
    context: Option<String>,
    id: String,
    plural: Option<String>,
    strs: Vec<String>,
}

/// A .po file's entries (gettext's format: keywords followed by quoted strings, continued on the next lines;
/// comments skipped). The header is the entry with no string.
fn entries(po: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut entry = Entry::default();
    // (The field the quoted lines go to: 0 context, 1 id, 2 plural, 3+ the translations.)
    let mut field = None;
    let mut seen_id = false;
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
                if seen_id {
                    out.push(std::mem::take(&mut entry));
                    seen_id = false;
                }
                entry.context = Some(text);
                field = Some(0);
            }
            "msgid" => {
                if seen_id {
                    out.push(std::mem::take(&mut entry));
                }
                seen_id = true;
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
    if seen_id {
        out.push(entry);
    }
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

/// A catalog of `entries` under `header` (its lines).
fn catalog<'a>(header: &str, entries: impl Iterator<Item = &'a Entry>) -> String {
    let mut out = String::from("msgid \"\"\nmsgstr \"\"\n");
    for line in header.lines() {
        let _ = writeln!(out, "{}", quote(&format!("{line}\n")));
    }
    out.push('\n');
    for e in entries {
        if let Some(c) = &e.context {
            let _ = writeln!(out, "msgctxt {}", quote(c));
        }
        let _ = writeln!(out, "msgid {}", quote(&e.id));
        match &e.plural {
            Some(p) => {
                let _ = writeln!(out, "msgid_plural {}", quote(p));
                for (i, s) in e.strs.iter().enumerate() {
                    let _ = writeln!(out, "msgstr[{i}] {}", quote(s));
                }
            }
            None => {
                let _ = writeln!(out, "msgstr {}", quote(e.strs.first().map_or("", String::as_str)));
            }
        }
        out.push('\n');
    }
    out
}

/// A string made longer (each vowel doubled, and brackets: about 40% more) and accented, its placeholders
/// (`{}`, `{0}`, `{n}`) kept as they are.
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
