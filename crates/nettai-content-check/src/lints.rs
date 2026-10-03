//! Static checks the type checker can't make (docs/design/content-model-v2.md
//! §7.7): lints for placeholder asset names, modules under `compat/`, kind
//! keys not qualified by their owner's folder, and table constants passed on
//! without a type.
//!
//! The checks read the source through a small scanner: comments are
//! dropped and string contents masked, so a pattern never matches inside
//! either, and positions are kept.

use crate::Problem;

/// A module's source with comments blanked and string contents masked
/// (quotes kept), byte for byte.
pub struct Scanned<'a> {
    pub source: &'a str,
    pub code: String,
}

impl Scanned<'_> {
    pub fn new(source: &str) -> Scanned<'_> {
        let b = source.as_bytes();
        let mut code = Vec::with_capacity(b.len());
        let mut i = 0;
        // The level of a long bracket at `i` (`[[`, `[=[`...), if one starts
        // there.
        let long_open = |i: usize| -> Option<usize> {
            if b.get(i) != Some(&b'[') {
                return None;
            }
            let mut j = i + 1;
            while b.get(j) == Some(&b'=') {
                j += 1;
            }
            (b.get(j) == Some(&b'[')).then_some(j - i - 1)
        };
        let long_close = |from: usize, level: usize| -> usize {
            let close: Vec<u8> = std::iter::once(b']').chain(std::iter::repeat_n(b'=', level)).chain([b']']).collect();
            b[from..].windows(close.len()).position(|w| w == close).map_or(b.len(), |p| from + p + close.len())
        };
        let blank = |code: &mut Vec<u8>, from: usize, to: usize, keep_ends: bool| {
            for (k, &c) in b[from..to].iter().enumerate() {
                let end = keep_ends && (k == 0 || from + k == to - 1);
                code.push(if c == b'\n' { b'\n' } else if end { c } else { b' ' });
            }
        };
        while i < b.len() {
            if b[i] == b'-' && b.get(i + 1) == Some(&b'-') {
                let end = match long_open(i + 2) {
                    Some(level) => long_close(i + 2, level),
                    None => b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |p| i + p),
                };
                blank(&mut code, i, end, false);
                i = end;
            } else if b[i] == b'"' || b[i] == b'\'' || b[i] == b'`' {
                let q = b[i];
                let mut j = i + 1;
                while j < b.len() && b[j] != q && b[j] != b'\n' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                let end = (j + 1).min(b.len());
                blank(&mut code, i, end, true);
                i = end;
            } else if let Some(level) = long_open(i) {
                let end = long_close(i, level);
                blank(&mut code, i, end, true);
                i = end;
            } else {
                code.push(b[i]);
                i += 1;
            }
        }
        Scanned { source, code: String::from_utf8_lossy(&code).into_owned() }
    }

    /// The line (from 1) of byte `at`.
    pub fn line(&self, at: usize) -> usize {
        self.source[..at].bytes().filter(|&c| c == b'\n').count() + 1
    }

    /// Every place `pattern` starts in the code, where it isn't the tail of
    /// a longer name.
    pub fn find<'p>(&'p self, pattern: &'p str) -> impl Iterator<Item = usize> + 'p {
        self.code.match_indices(pattern).map(|(at, _)| at).filter(move |&at| {
            let before = self.code[..at].chars().next_back();
            !(pattern.starts_with(|c: char| c.is_alphanumeric() || c == '_')
                && before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.' || c == ':'))
        })
    }

    /// The string literal starting at byte `at` (at its quote), unescaped
    /// naively.
    pub fn string_at(&self, at: usize) -> Option<&str> {
        let q = self.source[at..].chars().next()?;
        if !matches!(q, '"' | '\'') {
            return None;
        }
        let rest = &self.source[at + 1..];
        rest.find(q).map(|end| &rest[..end])
    }
}

/// The owner whose folder module `path` is in, if it is in one: a chip's
/// (`chips/<chip>/`), a navi's (`navis/<navi>/`), or, in a navi's folder,
/// a form's (`navis/<navi>/forms/<form>/`).
fn owner(path: &str) -> Option<&str> {
    let (top, rest) =
        ["chips/", "navis/", "forms/", "weapons/", "stages/"].iter().find_map(|top| Some((*top, path.strip_prefix(top)?)))?;
    let mut parts = rest.split('/');
    let first = parts.next()?;
    let inside: Vec<&str> = parts.collect();
    // A module right under the top folder has no owner.
    let _file = inside.last()?;
    if top != "navis/" {
        return Some(first);
    }
    if let ["forms", form, _, ..] = inside[..] {
        return Some(form);
    }
    Some(first)
}

/// Module-level table constants (`local SPEC = { ... }`) that are passed to
/// a function without a type annotation: (the constant's line, its name,
/// the line it is passed on).
///
/// The checker can't check such a table's shape. A table literal bound
/// without an annotation is unsealed, so it passes for any table type whose
/// required fields it has: a misspelled optional field isn't an error. And
/// a function of another module is `any` to the checker (it checks each
/// module on its own), so nothing at all is checked there. With the
/// annotation (`local SPEC: HeatFlame = { ... }`, the type one of the
/// pack's shared types in `types.d.luau`), the literal is checked where it
/// is written.
pub fn untyped_constants(s: &Scanned) -> Vec<(usize, String, usize)> {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let code = s.code.as_str();
    let mut out = Vec::new();
    let mut at = 0;
    for line in code.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let Some(rest) = line.strip_prefix("local ") else { continue };
        let name: &str = &rest[..rest.find(|c: char| !ident(c)).unwrap_or(rest.len())];
        // `local NAME = {`: one name, no annotation, a table literal.
        let after = rest[name.len()..].trim_start();
        if name.is_empty() || !after.strip_prefix('=').is_some_and(|v| v.trim_start().starts_with('{')) {
            continue;
        }
        let declared = start + "local ".len();
        let passed = s.find(name).find(|&u| {
            let (before, following) = (code[..u].trim_end(), code[u + name.len()..].trim_start());
            if u == declared || code[u + name.len()..].starts_with(ident) {
                return false;
            }
            // An argument by itself: between `(` or `,` and `,` or `)`.
            if !before.ends_with(['(', ',']) || !following.starts_with([')', ',']) {
                return false;
            }
            // Of a call: the innermost bracket open here is a `(` after a
            // name or a closing bracket, and not a function definition's.
            let mut depth = 0;
            let open = code[..u].char_indices().rev().find(|&(_, c)| match c {
                ')' | '}' | ']' => {
                    depth += 1;
                    false
                }
                '(' | '{' | '[' if depth > 0 => {
                    depth -= 1;
                    false
                }
                '(' | '{' | '[' => true,
                _ => false,
            });
            let Some((open, '(')) = open else { return false };
            let callee = code[..open].trim_end();
            if !callee.ends_with(|c: char| ident(c) || c == ')' || c == ']') {
                return false;
            }
            let head = &code[code[..open].rfind('\n').map_or(0, |n| n + 1)..open];
            !head.rfind("function").is_some_and(|f| !head[f..].contains('('))
        });
        if let Some(u) = passed {
            out.push((s.line(declared), name.to_string(), s.line(u)));
        }
    }
    out
}

/// The lints for module `path` (relative to the pack root, with `.luau`).
pub fn lints(path: &str, source: &str) -> Vec<Problem> {
    let mut out = Vec::new();
    if path.starts_with("compat/") {
        out.push(format!("{path}: compat holds the original's numbers as TOML; no module lives under compat/"));
    }
    let s = Scanned::new(source);
    // Placeholder asset names (`sprite-0c-01`): name the asset first.
    for kind in ["sprite", "sound", "banner", "background", "mugshot"] {
        let call = format!("asset.{kind}(");
        for at in s.find(&call) {
            let arg = at + call.len();
            let name = s.source[arg..].trim_start();
            let quote = arg + (s.source[arg..].len() - name.len());
            if let Some(name) = s.string_at(quote)
                && name.strip_prefix(kind).and_then(|r| r.strip_prefix('-')).is_some_and(|r| {
                    !r.is_empty() && r.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
                })
            {
                out.push(format!(
                    "{path}:{}: {name:?} is a placeholder; name the asset in compat/assets.toml first",
                    s.line(at)
                ));
            }
        }
    }
    for (line, name, passed) in untyped_constants(&s) {
        out.push(format!(
            "{path}:{line}: the table constant `{name}` is passed on (line {passed}) without a type: \
             annotate it (`local {name}: <its type> = {{ ... }}`) so the checker checks its shape"
        ));
    }
    // A system's state is its own (docs/design/rules-in-luau.md §4.5): only
    // a game's rules (modules under rules/) reach it.
    if !path.starts_with("rules/") {
        for call in ["system.state(", "system.setup(", "system.side("] {
            for at in s.find(call) {
                out.push(format!(
                    "{path}:{}: `{}` is a system's own: only modules under rules/ call it; content reaches a game's rules \
                     through its API module",
                    s.line(at),
                    &call[..call.len() - 1]
                ));
            }
        }
    }
    // A kind in an owner's folder is keyed under its owner.
    if let Some(owner) = owner(path) {
        for at in s.find("define.kind") {
            let rest = &s.code[at..];
            let Some(id) = rest.find("id").filter(|&i| i < rest.find('}').unwrap_or(rest.len())) else { continue };
            let after = &s.code[at + id + 2..];
            let Some(eq) = after.find('=') else { continue };
            let quote = at + id + 2 + eq + 1 + (after[eq + 1..].len() - after[eq + 1..].trim_start().len());
            if let Some(key) = s.string_at(quote)
                && !key.split_once(':').map_or(key, |(_, k)| k).starts_with(&format!("{owner}/"))
            {
                out.push(format!("{path}:{}: kind {key:?} lives in {owner}'s folder: key it \"{owner}/...\"", s.line(at)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_strings_are_not_code() {
        let s = Scanned::new("local x = 'me:set_action(1)' -- me:set_action(2)\n--[[ me:set_action(3) ]] me:set_action(4)\n");
        assert_eq!(s.find(":set_action(").count(), 1);
        assert_eq!(s.code.len(), s.source.len());
    }

    #[test]
    fn a_table_constant_passed_on_needs_its_type() {
        let flagged = |src: &str| -> Vec<String> { untyped_constants(&Scanned::new(src)).into_iter().map(|(_, n, _)| n).collect() };
        // Passed to a function (another module's is `any` to the checker; a
        // typed one of this module takes an unsealed table as it comes).
        assert_eq!(flagged("local FLAME = { ticks = 30 }\nflame.spawn(me, x, y, FLAME, damage)\n"), ["FLAME"]);
        assert_eq!(flagged("local PHASES = { [0] = a, [4] = b }\nlocal function f(me)\n    run(me, PHASES)\nend\n"), ["PHASES"]);
        assert_eq!(flagged("local A = {\n    1,\n    2,\n}\npick(A)\n"), ["A"]);
        // Annotated, it is checked where it is written.
        assert!(flagged("local FLAME: HeatFlame = { ticks = 30 }\nflame.spawn(me, x, y, FLAME, damage)\n").is_empty());
        // Not passed on: indexed, iterated, a field of another table, a
        // definition's argument inline, a parameter of the same name.
        for src in [
            "local T = { 1, 2 }\nlocal x = T[1]\n",
            "local T = { 1, 2 }\nfor _, v in T do print(v) end\n",
            "local T = { 1, 2 }\nlocal U = { T, 3 }\n",
            "local T = { 1, 2 }\nlocal x = f(T[1], T.n)\n",
            "local K = define.kind { id = 'k' }\nbattle.spawn(K, pos)\n",
            "local T = { 1 }\nlocal function f(a, T)\nend\n",
            "local t = 3\nf(t)\n",
            "    local T = { 1 }\n    f(T)\n",
        ] {
            assert!(flagged(src).is_empty(), "{src}");
        }
        let l = lints("lib/x.luau", "local FLAME = { ticks = 30 }\nflame.spawn(FLAME)\n");
        assert!(l.len() == 1 && l[0].starts_with("lib/x.luau:1: the table constant `FLAME`"), "{l:?}");
    }

    #[test]
    fn lints_catch_placeholders_compat_modules_and_unqualified_kinds() {
        let l = lints("chips/minibomb/chip.luau", "local S = asset.sprite('sprite-0c-01')\nlocal K = define.kind { id = 'bomb', pool = 'attack' }\n");
        assert_eq!(l.len(), 2, "{l:?}");
        assert!(l[0].contains("placeholder") && l[1].contains("minibomb/"));
        assert!(lints("chips/minibomb/chip.luau", "local K = define.kind { id = 'minibomb/held', pool = 'effect' }").is_empty());
        assert!(lints("lib/bombs/bomb.luau", "local K = define.kind { id = 'bomb', pool = 'attack' }").is_empty());
        // A navi's folder (without its index prefix), and a form's inside it.
        let dash = "local K = define.kind { id = 'megaman/dash-hit', pool = 'attack' }";
        assert!(lints("navis/megaman/dash_hit.luau", dash).is_empty());
        assert!(lints("navis/megaman/dash_hit.luau", dash).is_empty());
        let wave = "local K = define.kind { id = 'slashcross/sword-wave', pool = 'attack' }";
        assert!(lints("navis/megaman/forms/slashcross/sword_wave.luau", wave).is_empty());
        assert_eq!(lints("navis/megaman/forms/heatcross/sword_wave.luau", wave).len(), 1);
        assert_eq!(lints("navis/megaman/sword_wave.luau", wave).len(), 1);
        assert_eq!(lints("compat/x.luau", "").len(), 1);
        // A system's state, outside the rules.
        let l = lints("chips/x/chip.luau", "local s = system.state()\n");
        assert!(l.len() == 1 && l[0].contains("`system.state` is a system's own"), "{l:?}");
        assert!(lints("rules/beast/system.luau", "local s = system.state()\nlocal u = system.setup()\n").is_empty());
        // A form's kinds are keyed under the form, and a navi's under its
        // name, whatever number its folder still carries.
        let surge = "local K = define.kind { id = 'spoutcross-beast/surge', pool = 'attack' }";
        assert!(lints("navis/megaman/forms/spoutcross-beast/surge.luau", surge).is_empty());
        assert_eq!(lints("navis/megaman/forms/tengucross-beast/surge.luau", surge).len(), 1);
        let shared = "local K = define.kind { id = 'megaman/dash-hit', pool = 'attack' }";
        assert!(lints("navis/megaman/dash_hit.luau", shared).is_empty());
        assert!(lints("navis/megaman/dash_hit.luau", shared).is_empty());
        assert_eq!(lints("navis/heatman/dash_hit.luau", shared).len(), 1);
    }
}
