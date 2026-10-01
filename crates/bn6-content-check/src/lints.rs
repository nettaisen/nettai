//! Static checks the type checker can't make (docs/design/content-model-v2.md
//! §7.7): uses of the numeric API that the v2 API replaces, counted per
//! module against an allowance that only shrinks (the ratchet, §12), and
//! lints (placeholder asset names, modules under `compat/`, kind keys not
//! qualified by their owner's folder).
//!
//! The checks read the source through a small scanner: comments are
//! dropped and string contents masked, so a pattern never matches inside
//! either, and positions are kept.

use std::collections::BTreeSet;

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

    /// The arguments of the call whose `(` is at byte `open`: each
    /// argument's code, trimmed.
    pub fn args(&self, open: usize) -> Vec<&str> {
        let code = &self.code[open + 1..];
        let (mut depth, mut start, mut out) = (0i32, 0, Vec::new());
        for (i, c) in code.char_indices() {
            match c {
                '(' | '{' | '[' => depth += 1,
                ')' | '}' | ']' if depth == 0 => {
                    out.push(code[start..i].trim());
                    break;
                }
                ')' | '}' | ']' => depth -= 1,
                ',' if depth == 0 => {
                    out.push(code[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.retain(|a| !a.is_empty());
        out
    }

    /// Module-level numeric constants (`local ANIM_THROW, TICKS = 6, 0x15`).
    pub fn numeric_constants(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for line in self.code.lines() {
            let Some(rest) = line.strip_prefix("local ") else { continue };
            let Some((names, values)) = rest.split_once('=') else { continue };
            let names: Vec<&str> = names.split(',').map(|n| n.split(':').next().unwrap_or("").trim()).collect();
            let values: Vec<&str> = values.split(',').map(str::trim).collect();
            for (n, v) in names.iter().zip(&values) {
                if is_number(v) {
                    out.insert(n.to_string());
                }
            }
        }
        out
    }
}

/// Whether `s` is a numeric literal (decimal or hex, maybe negative).
fn is_number(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s).trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return !hex.is_empty() && hex.bytes().all(|c| c.is_ascii_hexdigit() || c == b'_');
    }
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit() || c == b'_')
}

/// A use of the numeric API that the v2 API replaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deprecated {
    pub line: usize,
    pub what: &'static str,
    pub instead: &'static str,
}

/// Calls deprecated whatever their arguments: pattern, what, instead.
const ALWAYS: &[(&str, &str, &str)] = &[
    ("battle.spawn_kind(", "battle.spawn_kind", "battle.spawn(kind, pos)"),
    ("battle.spawn_kind_first(", "battle.spawn_kind_first", "battle.spawn_first(kind, pos)"),
    ("battle.spawn_kind_at_end(", "battle.spawn_kind_at_end", "battle.spawn_at_end(kind, pos)"),
    (":param(", "me:param", "the kind's state"),
    (":set_param(", "me:set_param", "the kind's state"),
    (":attack_param(", "me:attack_param", "the builder's arguments"),
    (":set_attack_param(", "me:set_attack_param", "the builder's arguments"),
    ("me.index", "me.index", "me.kind"),
    ("me.variant", "me.variant", "the builder's arguments"),
    (".name_id", "name_id", "the identity (step 11)"),
    ("battle.navi_record(", "battle.navi_record", "the identity (step 11)"),
    (":death_hook(", "me:death_hook", "the identity (step 11)"),
    ("battle.attach_point(", "battle.attach_point", "me:attach_point_pos"),
    (":lockon_panel(", "me:lockon_panel", "a lock-on mode definition"),
    ("battle.hand_chip(", "battle.hand_chip", "chips by handle (step 3b)"),
    ("data.", "the data global", "definitions"),
    (":load(\"", "sprite:load(id)", "asset.sprite"),
    ("legacy {", "a legacy marker", "the v2 form it stands for"),
    ("legacy = {", "a legacy marker", "the v2 form it stands for"),
];

/// Calls deprecated when an argument is a number: pattern, the arguments
/// (0-based) that take a definition, what, instead.
const BY_ARGUMENT: &[(&str, &[usize], &str, &str)] = &[
    ("battle.play_sound(", &[0], "battle.play_sound(number)", "asset.sound"),
    ("battle.play_sound_for(", &[1], "battle.play_sound_for(side, number)", "asset.sound"),
    ("battle.effect(", &[1], "battle.effect(pos, number)", "define.effect"),
    ("battle.spark(", &[2], "battle.spark(owner, pos, number)", "define.spark"),
    ("battle.region_effects(", &[2, 4], "battle.region_effects(number)", "define.region, define.effect"),
    (":setup_collision(", &[0, 1], "me:setup_collision(number)", "define.collision"),
    (":reset_collision_types(", &[0, 1], "me:reset_collision_types(number)", "define.collision"),
    (":set_attack(", &[0], "me:set_attack(number)", "an action definition"),
];

/// Every deprecated use in a module.
pub fn deprecated(source: &str) -> Vec<Deprecated> {
    let s = Scanned::new(source);
    let constants = s.numeric_constants();
    let numeric = |a: &str| is_number(a) || constants.contains(a);
    let mut out = Vec::new();
    for &(pattern, what, instead) in ALWAYS {
        for at in s.find(pattern) {
            out.push(Deprecated { line: s.line(at), what, instead });
        }
    }
    for at in s.find("battle.spawn(") {
        if s.args(at + "battle.spawn".len()).first().is_some_and(|a| a.starts_with('"') || a.starts_with('\'')) {
            out.push(Deprecated { line: s.line(at), what: "battle.spawn(pool, index)", instead: "battle.spawn(kind, pos)" });
        }
    }
    // A weapon definition's setup that names its action by number (the
    // v1 modules' `return ACTION`): a numeric constant whose name says it
    // is an action.
    if s.find("define.weapon").next().is_some() {
        for at in s.find("return ") {
            let name = s.code[at + "return ".len()..].split(|c: char| !(c.is_alphanumeric() || c == '_')).next().unwrap_or("");
            if name.contains("ACTION") && constants.contains(name) {
                out.push(Deprecated { line: s.line(at), what: "an action by number", instead: "an action definition" });
            }
        }
    }
    for &(pattern, positions, what, instead) in BY_ARGUMENT {
        for at in s.find(pattern) {
            let args = s.args(at + pattern.len() - 1);
            if positions.iter().any(|&p| args.get(p).is_some_and(|a| numeric(a))) {
                out.push(Deprecated { line: s.line(at), what, instead });
            }
        }
    }
    out.sort_by_key(|d| d.line);
    out
}

/// The owner whose folder module `path` is in, if it is in one: a chip's
/// (`chips/<chip>/`), a navi's (`navis/<navi>/`, its index prefix, if it
/// still has one, left out: `navis/00-megaman/` is `megaman`'s), or, in a
/// navi's folder, a form's (`navis/<navi>/forms/<form>/`).
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
    let index = first.split_once('-').filter(|(n, _)| !n.is_empty() && n.bytes().all(|c| c.is_ascii_hexdigit()));
    Some(index.map_or(first, |(_, name)| name))
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
    // A kind in an owner's folder is keyed under its owner.
    if let Some(owner) = owner(path) {
        for at in s.find("define.kind") {
            let rest = &s.code[at..];
            let Some(id) = rest.find("id").filter(|&i| i < rest.find('}').unwrap_or(rest.len())) else { continue };
            let after = &s.code[at + id + 2..];
            let Some(eq) = after.find('=') else { continue };
            let quote = at + id + 2 + eq + 1 + (after[eq + 1..].len() - after[eq + 1..].trim_start().len());
            if let Some(key) = s.string_at(quote)
                && !key.starts_with(&format!("{owner}/"))
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
        let s = Scanned::new("local x = 'me:param(1)' -- me:param(2)\n--[[ me:param(3) ]] local y = me:param(4)\n");
        assert_eq!(s.find(":param(").count(), 1);
        assert_eq!(s.code.len(), s.source.len());
    }

    #[test]
    fn numeric_uses_count_and_definitions_do_not() {
        let src = "local SOUND, ANIM = 0x1A6, 6\n\
                   local THROW = asset.sound('throw')\n\
                   battle.play_sound(SOUND)\n\
                   battle.play_sound(THROW)\n\
                   battle.play_sound(0x10)\n\
                   local o = battle.spawn('attack', 8, me.pos)\n\
                   local k = battle.spawn(bomb.kind, me.pos)\n\
                   me:set_attack(ACTION, 0)\n\
                   me:set_attack(0x12, 0)\n\
                   local _ = data.chips[1]\n";
        let d: Vec<&str> = deprecated(src).iter().map(|d| d.what).collect();
        assert_eq!(
            d,
            ["battle.play_sound(number)", "battle.play_sound(number)", "battle.spawn(pool, index)", "me:set_attack(number)", "the data global"]
        );
    }

    #[test]
    fn a_weapon_definition_naming_its_action_by_number_counts() {
        let setup = "local ACTION, DAMAGE = 0x27, 30\n\
                     local function setup(navi: Object): number\n    return ACTION\nend\n";
        let weapon = format!("{setup}local W = define.weapon {{ id = 'w', name = 'W', charge_ticks = {{}}, setup = setup }}\n");
        let d: Vec<&str> = deprecated(&weapon).iter().map(|d| d.what).collect();
        assert_eq!(d, ["an action by number"]);
        // (A v1 weapon module's whole registration is the transitional part.)
        assert!(deprecated(setup).is_empty());
        let named = weapon.replace("return ACTION", "return SHOT");
        assert!(deprecated(&named).is_empty());
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
        assert!(lints("navis/00-megaman/dash_hit.luau", dash).is_empty());
        assert!(lints("navis/megaman/dash_hit.luau", dash).is_empty());
        let wave = "local K = define.kind { id = 'slashcross/sword-wave', pool = 'attack' }";
        assert!(lints("navis/00-megaman/forms/slashcross/sword_wave.luau", wave).is_empty());
        assert_eq!(lints("navis/00-megaman/forms/heatcross/sword_wave.luau", wave).len(), 1);
        assert_eq!(lints("navis/00-megaman/sword_wave.luau", wave).len(), 1);
        assert_eq!(lints("compat/x.luau", "").len(), 1);
    }
}
