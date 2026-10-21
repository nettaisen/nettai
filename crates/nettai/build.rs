//! The software the app is made of, for its About screen (docs/app.md §18): every package built into the app
//! for the target being built, each with its name, version, license, repository and the license files it
//! ships, and the fonts the UI bundles with theirs. The packages are those `cargo tree -p nettai` lists, with
//! the features a build of the app alone gives them (not the tools that build or test it), and the sources of
//! a library a build script compiles into it (a `*-src` package: SDL's, Luau's); `cargo metadata` says where
//! each is. Written to `licenses.rs`: the texts once each (many packages ship the same), the packages by
//! name, each with its files' names and texts. The workspace's own crates are nettai's, and are none of it.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The fonts the UI and the battle's drawing bundle, each with its license and its license file (from the
/// workspace's root). The digit fonts are nettai's own (the game's digits traced: their folders' READMEs), under
/// the repository's license.
const FONTS: [(&str, &str, &str); 7] = [
    ("Bungee", "OFL-1.1", "crates/nettai-render/fonts/bungee/OFL.txt"),
    ("Chakra Petch", "OFL-1.1", "crates/nettai-ui/fonts/chakra-petch/OFL.txt"),
    ("Corporate Logo", "OFL-1.1", "crates/nettai-ui/fonts/corporate-logo/OFL.txt"),
    ("M PLUS 1 Code", "OFL-1.1", "crates/nettai-render/fonts/mplus1code/OFL.txt"),
    ("Murecho", "OFL-1.1", "crates/nettai-render/fonts/murecho/OFL.txt"),
    ("Nettai Banner Digits", "MPL-2.0", "LICENSE"),
    ("Nettai Digits", "MPL-2.0", "LICENSE"),
];

/// A package's license files: by name, in its folder, in a `LICENSES` folder, or in a folder of its own (a
/// library it builds from source keeps its license there: SDL's, Luau's).
fn is_license_file(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    ["LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE"].iter().any(|start| name.starts_with(start))
}

fn license_files(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return found };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if path.is_file() && is_license_file(&name) {
            found.push(path);
        } else if path.is_dir() {
            let inner = std::fs::read_dir(&path).into_iter().flatten().flatten();
            let mut inner: Vec<PathBuf> = inner.map(|e| e.path()).filter(|p| p.is_file()).collect();
            inner.sort();
            let all = name.eq_ignore_ascii_case("LICENSES");
            found.extend(inner.into_iter().filter(|p| all || p.file_name().is_some_and(|n| is_license_file(&n.to_string_lossy()))));
            // (And the libraries that library vendors in its `deps`, built into it too: libdatachannel's ICE,
            // SCTP and logging, in libdatachannel-sys.)
            let mut vendored: Vec<PathBuf> = std::fs::read_dir(path.join("deps")).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            vendored.sort();
            for dep in vendored {
                let mut files: Vec<PathBuf> = std::fs::read_dir(&dep).into_iter().flatten().flatten().map(|e| e.path()).collect();
                files.sort();
                found.extend(files.into_iter().filter(|p| p.is_file() && p.file_name().is_some_and(|n| is_license_file(&n.to_string_lossy()))));
            }
        }
    }
    found
}

struct Package {
    name: String,
    version: String,
    license: String,
    repository: String,
    /// Each license file: its name in the package, and its text's place in the texts.
    texts: Vec<(String, usize)>,
}

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let root = manifest.join("../..");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets it"));
    let target = std::env::var("TARGET").expect("cargo sets it");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", root.join("Cargo.lock").display());
    println!("cargo:rerun-if-changed={}", manifest.join("Cargo.toml").display());
    for (_, _, file) in FONTS {
        println!("cargo:rerun-if-changed={}", root.join(file).display());
    }

    let run = |args: &[&str]| -> Vec<u8> {
        let output = std::process::Command::new(&cargo).args(args).arg("--manifest-path").arg(root.join("Cargo.toml")).output().expect("cargo runs");
        assert!(output.status.success(), "cargo {} failed: {}", args[0], String::from_utf8_lossy(&output.stderr));
        output.stdout
    };
    // (Each package as `cargo tree` prints it: "name vVersion", then where it is from unless the registry.)
    let tree = |edges: &str| -> BTreeSet<(String, String)> {
        let out = run(&["tree", "-p", "nettai", "-e", edges, "--target", &target, "--prefix", "none", "-f", "{p}", "--offline"]);
        String::from_utf8_lossy(&out)
            .lines()
            .filter_map(|line| {
                let mut words = line.split(' ');
                Some((words.next()?.to_string(), words.next()?.strip_prefix('v')?.to_string()))
            })
            .collect()
    };
    let mut built = tree("normal");
    built.extend(tree("normal,build").into_iter().filter(|(name, _)| name.ends_with("-src")));

    let metadata: serde_json::Value = serde_json::from_slice(&run(&["metadata", "--format-version", "1", "--offline", "--filter-platform", &target])).expect("cargo metadata's JSON");
    let members: BTreeSet<&str> = metadata["workspace_members"].as_array().expect("members").iter().filter_map(|m| m.as_str()).collect();

    let mut texts: Vec<String> = Vec::new();
    let mut text_of = |text: String| -> usize {
        let text = text.replace("\r\n", "\n").trim().to_string();
        match texts.iter().position(|t| *t == text) {
            Some(at) => at,
            None => {
                texts.push(text);
                texts.len() - 1
            }
        }
    };
    let mut list: Vec<Package> = Vec::new();
    for p in metadata["packages"].as_array().expect("packages") {
        let (name, version) = (p["name"].as_str().unwrap_or(""), p["version"].as_str().unwrap_or(""));
        if members.contains(p["id"].as_str().unwrap_or("")) || !built.contains(&(name.to_string(), version.to_string())) {
            continue;
        }
        let dir = Path::new(p["manifest_path"].as_str().expect("a manifest")).parent().expect("its folder").to_path_buf();
        let mut files = license_files(&dir);
        if let Some(file) = p["license_file"].as_str() {
            let file = dir.join(file);
            if !files.contains(&file) {
                files.push(file);
            }
        }
        let texts = files
            .iter()
            .filter_map(|f| {
                let name = f.strip_prefix(&dir).unwrap_or(f).to_string_lossy().replace('\\', "/");
                std::fs::read(f).ok().map(|b| (name, text_of(String::from_utf8_lossy(&b).into_owned())))
            })
            .collect();
        list.push(Package {
            name: name.to_string(),
            version: version.to_string(),
            license: p["license"].as_str().unwrap_or("").to_string(),
            repository: p["repository"].as_str().unwrap_or("").to_string(),
            texts,
        });
    }
    list.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()).then_with(|| a.version.cmp(&b.version)));
    let fonts: Vec<Package> = FONTS
        .iter()
        .map(|(name, license, file)| {
            let text = std::fs::read_to_string(root.join(file)).unwrap_or_else(|e| panic!("can't read {file}: {e}"));
            let shown = Path::new(file).file_name().map_or_else(|| file.to_string(), |n| n.to_string_lossy().into_owned());
            Package { name: name.to_string(), version: String::new(), license: license.to_string(), repository: String::new(), texts: vec![(shown, text_of(text))] }
        })
        .collect();

    let mut rs = String::from("// Written by build.rs: the software the app is made of (docs/app.md §18).\n\n");
    let write_list = |rs: &mut String, name: &str, list: &[Package]| {
        writeln!(rs, "pub static {name}: &[Software] = &[").unwrap();
        for p in list {
            writeln!(rs, "    Software {{ name: {:?}, version: {:?}, license: {:?}, repository: {:?}, texts: &{:?} }},", p.name, p.version, p.license, p.repository, p.texts).unwrap();
        }
        writeln!(rs, "];\n").unwrap();
    };
    write_list(&mut rs, "PACKAGES", &list);
    write_list(&mut rs, "FONTS", &fonts);
    writeln!(rs, "pub static TEXTS: &[&str] = &[").unwrap();
    for text in &texts {
        writeln!(rs, "    {text:?},").unwrap();
    }
    writeln!(rs, "];").unwrap();
    std::fs::write(out.join("licenses.rs"), rs).expect("licenses.rs written");

    // The same as a text file, which a packaged app ships beside the program (packaging/: it finds the file
    // through cargo's `build-script-executed` message, which names this script's OUT_DIR).
    let mut txt = String::from("The software nettai is made of, each with its license (also on its About screen).\n\n");
    for p in fonts.iter().chain(&list) {
        let version = if p.version.is_empty() { String::new() } else { format!(" {}", p.version) };
        writeln!(txt, "{}{version}: {}{}", p.name, p.license, if p.repository.is_empty() { String::new() } else { format!(", {}", p.repository) }).unwrap();
        for (file, at) in &p.texts {
            writeln!(txt, "    {file}: text {}", at + 1).unwrap();
        }
    }
    for (at, text) in texts.iter().enumerate() {
        writeln!(txt, "\n\n==== Text {} ====\n\n{text}", at + 1).unwrap();
    }
    std::fs::write(out.join("licenses.txt"), txt).expect("licenses.txt written");

    windows_icon();
}

/// The Windows program's icon, from the .ico `$NETTAI_WINDOWS_ICON` names (packaging/icons.sh makes it from
/// docs/logo/nettai-icon.svg). Without it the program has no icon of its own: a build for development.
fn windows_icon() {
    println!("cargo:rerun-if-env-changed=NETTAI_WINDOWS_ICON");
    let Some(icon) = std::env::var_os("NETTAI_WINDOWS_ICON") else { return };
    println!("cargo:rerun-if-changed={}", Path::new(&icon).display());
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    #[cfg(windows)]
    winresource::WindowsResource::new().set_icon(&icon.to_string_lossy()).compile().expect("the icon's resource compiles");
    #[cfg(not(windows))]
    println!("cargo:warning=NETTAI_WINDOWS_ICON is set, but a Windows program's icon is compiled on Windows only");
}
