//! Which files of a folder the player chose are wanted: the paths [`crate::open_folder`] takes, matched as the
//! browser matches them (a phone reads them out of the folder: `phone`).

/// Whether `path` (in a folder chosen: its parts by `/`) is at one of `wanted` (folders by name, any case; the
/// file's name with `*` for any letters and `?` for one, any case), as the browser finds them.
pub(crate) fn is_wanted(path: &str, wanted: &[String]) -> bool {
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    wanted.iter().any(|w| {
        let want: Vec<&str> = w.split('/').filter(|p| !p.is_empty()).collect();
        want.len() == parts.len()
            && want.iter().zip(&parts).enumerate().all(|(at, (w, p))| if at + 1 == want.len() { glob(w, p) } else { w.eq_ignore_ascii_case(p) })
    })
}

/// How deep in a folder chosen a wanted file may be (`exe/data/exe4.dat`).
pub(crate) fn depth(wanted: &[String]) -> usize {
    wanted.iter().map(|w| w.split('/').filter(|p| !p.is_empty()).count()).max().unwrap_or(0)
}

/// `name` matched against `pattern`: `*` any letters, `?` one, the rest any case.
fn glob(pattern: &str, name: &str) -> bool {
    fn at(p: &[char], n: &[char]) -> bool {
        match p.split_first() {
            None => n.is_empty(),
            Some(('*', rest)) => (0..=n.len()).any(|skip| at(rest, &n[skip..])),
            Some(('?', rest)) => !n.is_empty() && at(rest, &n[1..]),
            Some((c, rest)) => n.first().is_some_and(|f| f.eq_ignore_ascii_case(c)) && at(rest, &n[1..]),
        }
    }
    at(&pattern.chars().collect::<Vec<_>>(), &name.chars().collect::<Vec<_>>())
}

/// The files of the folder at `root` that are `wanted`, by their paths in it (iOS reads a folder chosen as files).
#[cfg(any(ios, test))]
pub(crate) fn wanted_in(root: &std::path::Path, wanted: &[String]) -> Vec<String> {
    fn walk(dir: &std::path::Path, at: &str, left: usize, wanted: &[String], out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if at.is_empty() { name } else { format!("{at}/{name}") };
            match entry.file_type() {
                Ok(t) if t.is_dir() && left > 1 => walk(&entry.path(), &path, left - 1, wanted, out),
                Ok(t) if t.is_file() && is_wanted(&path, wanted) => out.push(path),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", depth(wanted), wanted, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder chosen is read as the browser reads one: its folders by name in any case, the file's name by its
    /// pattern, at the pattern's depth alone.
    #[test]
    fn a_folder_chosen_is_read_by_the_wanted_paths() {
        let wanted: Vec<String> = ["exe/data/exe?*.dat", "data/exe?*.dat", "exe?*.dat"].map(String::from).to_vec();
        assert!(is_wanted("exe/data/exe4.dat", &wanted));
        assert!(is_wanted("EXE/Data/EXE6.DAT", &wanted));
        assert!(is_wanted("data/exe5.dat", &wanted));
        assert!(is_wanted("exe6.dat", &wanted));
        assert!(!is_wanted("exe.dat", &wanted), "the shared archive's name has no letter after exe");
        assert!(!is_wanted("exe/exe4.dat", &wanted));
        assert!(!is_wanted("exe/data/other/exe4.dat", &wanted));
        assert_eq!(depth(&wanted), 3);
    }

    /// Of a folder on disk, the wanted files are found at their depth, any case, and nothing else.
    #[test]
    fn the_wanted_files_of_a_folder_are_found() {
        let root = std::env::temp_dir().join(format!("nettai-dialogs-wanted-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for file in ["exe/data/exe4.dat", "exe/data/EXE6.DAT", "exe/data/exe.dat", "exe/data/deeper/exe5.dat", "exe/readme.txt"] {
            let path = root.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"x").unwrap();
        }
        let wanted: Vec<String> = ["exe/data/exe?*.dat", "data/exe?*.dat", "exe?*.dat"].map(String::from).to_vec();
        let mut found = wanted_in(&root, &wanted);
        found.sort();
        assert_eq!(found, ["exe/data/EXE6.DAT", "exe/data/exe4.dat"]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
