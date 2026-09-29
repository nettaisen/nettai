//! Load-time checks on compiled content: no function may keep state
//! between calls.
//!
//! Content functions are stateless (docs/design/scripting.md): everything
//! a script keeps between ticks lives in engine-owned state, so a battle
//! snapshot never has to include the VM. A Luau function could otherwise
//! keep state in three places:
//!
//! 1. **Globals.** Rejected here: no function may assign a global
//!    (`SETGLOBAL`). Modules export through their return value.
//! 2. **Module-level locals** captured by a function (`local n = 0` at the
//!    top of a file, then `n += 1` in `update`). Rejected here: no function
//!    may assign an upvalue that is a local of the module's main chunk
//!    (`SETUPVAL`, followed back through the `CAPTURE` chain). Assigning a
//!    local of an enclosing *function* is fine: that frame dies with the
//!    call.
//! 3. **Tables reachable from a module** (`local cache = {}` then
//!    `cache[k] = v`). Not a bytecode matter: the loader freezes every
//!    table reachable from a module's result and its functions' upvalues
//!    (`sandbox::deep_freeze`), so the write fails at run time.
//!
//! The check reads Luau's bytecode format as `lvmload.cpp` does (versions
//! 3..=14) and fails closed on anything it doesn't understand.

use std::collections::HashMap;
use std::fmt;

/// Opcodes this check looks at (`LuauOpcode`).
mod op {
    pub const SETGLOBAL: u8 = 8;
    pub const SETUPVAL: u8 = 10;
    pub const NEWCLOSURE: u8 = 19;
    pub const DUPCLOSURE: u8 = 64;
    pub const CAPTURE: u8 = 70;

    /// Opcodes followed by an auxiliary word (`getOpLength`).
    pub const WITH_AUX: [u8; 30] = [
        7, 8, 12, 15, 16, 20, 27, 28, 29, 30, 31, 32, 53, 55, 58, 66, 74, 75, 60, 77, 78, 79, 80, 83, 84, 85, 86, 87, 88,
        90,
    ];
}

/// `LCT_*`: how `CAPTURE` captures.
const CAPTURE_UPVAL: u8 = 2;

/// Why a module was rejected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub module: String,
    pub function: String,
    pub line: u32,
    pub rule: Rule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    AssignsGlobal,
    AssignsModuleLocal { name: String },
    /// The bytecode couldn't be read (the check fails closed).
    Unreadable(String),
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let at = format!("{}.luau: function `{}` (line {})", self.module, self.function, self.line);
        match &self.rule {
            Rule::AssignsGlobal => write!(f, "{at} assigns a global; content modules export through their return value"),
            Rule::AssignsModuleLocal { name } => write!(
                f,
                "{at} assigns the module-level local `{name}`; state that outlives a call must live in engine-owned fields \
                 (the object's or action's `state`)"
            ),
            Rule::Unreadable(why) => write!(f, "{}.luau: unreadable bytecode ({why})", self.module),
        }
    }
}

struct Proto {
    code: Vec<u32>,
    nups: u8,
    /// Constant index -> proto id, for closure constants.
    closures: HashMap<usize, usize>,
    children: Vec<usize>,
    name: Option<String>,
    line: u32,
    upvalue_names: Vec<String>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let b = *self.data.get(self.at).ok_or("truncated")?;
        self.at += 1;
        Ok(b)
    }

    fn skip(&mut self, n: usize) -> Result<(), String> {
        if self.at + n > self.data.len() {
            return Err("truncated".into());
        }
        self.at += n;
        Ok(())
    }

    fn u32(&mut self) -> Result<u32, String> {
        let b = self.data.get(self.at..self.at + 4).ok_or("truncated")?;
        self.at += 4;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }

    fn varint(&mut self) -> Result<u64, String> {
        let mut v = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.u8()?;
            v |= ((b & 0x7F) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
        }
        Err("varint too long".into())
    }

    fn size(&mut self) -> Result<usize, String> {
        Ok(self.varint()? as usize)
    }
}

fn parse(bytecode: &[u8]) -> Result<(Vec<Proto>, usize), String> {
    let mut r = Reader { data: bytecode, at: 0 };
    let version = r.u8()?;
    if version == 0 {
        return Err(String::from_utf8_lossy(&bytecode[1..]).into_owned());
    }
    if !(3..=14).contains(&version) {
        return Err(format!("bytecode version {version}"));
    }
    let types_version = if version >= 4 { r.u8()? } else { 0 };
    let strings: Vec<String> = (0..r.size()?)
        .map(|_| {
            let n = r.size()?;
            let s = r.data.get(r.at..r.at + n).ok_or("truncated")?;
            r.at += n;
            Ok(String::from_utf8_lossy(s).into_owned())
        })
        .collect::<Result<_, String>>()?;
    let string = |i: u64| if i == 0 { None } else { strings.get(i as usize - 1).cloned() };
    if types_version == 3 {
        while r.u8()? != 0 {
            r.varint()?;
        }
    }
    let mut protos = Vec::new();
    for _ in 0..r.size()? {
        let end = if version >= 12 {
            let n = r.size()?;
            Some(r.at + n)
        } else {
            None
        };
        r.skip(2)?; // max stack, params
        let nups = r.u8()?;
        r.skip(1)?; // vararg
        if version >= 4 {
            r.skip(1)?; // flags
            let n = r.size()?;
            r.skip(n)?; // type info
        }
        let code = (0..r.size()?).map(|_| r.u32()).collect::<Result<Vec<_>, _>>()?;
        let mut closures = HashMap::new();
        for k in 0..r.size()? {
            match r.u8()? {
                0 => {}
                1 => r.skip(1)?,
                2 => r.skip(8)?,
                3 => {
                    r.varint()?;
                }
                4 => r.skip(4)?,
                5 => {
                    for _ in 0..r.size()? {
                        r.varint()?;
                    }
                }
                6 => {
                    closures.insert(k, r.size()?);
                }
                7 => r.skip(16)?,
                8 => {
                    for _ in 0..r.size()? {
                        r.varint()?;
                        r.skip(4)?;
                    }
                }
                9 => {
                    r.skip(1)?;
                    r.varint()?;
                }
                10 => {
                    r.varint()?;
                    let n = r.size()? + r.size()?;
                    for _ in 0..n {
                        r.varint()?;
                    }
                }
                11 => r.skip(32)?,
                t => return Err(format!("constant type {t}")),
            }
        }
        let children = (0..r.size()?).map(|_| r.size()).collect::<Result<Vec<_>, _>>()?;
        let line = r.varint()? as u32;
        let name = string(r.varint()?);
        let mut upvalue_names = Vec::new();
        if let Some(end) = end {
            // Debug info follows; the size lets us skip what we don't need,
            // but we still want upvalue names, so read on when present.
            if let Ok(names) = read_debug(&mut r, code.len(), &string) {
                upvalue_names = names;
            }
            r.at = end;
        } else {
            upvalue_names = read_debug(&mut r, code.len(), &string)?;
            if version >= 11 {
                for _ in 0..r.size()? {
                    r.skip(1)?;
                    r.varint()?;
                }
            }
        }
        protos.push(Proto { code, nups, closures, children, name, line, upvalue_names });
    }
    let main = r.size()?;
    if main >= protos.len() {
        return Err("no main function".into());
    }
    Ok((protos, main))
}

/// Line and debug info; returns the upvalue names (if any).
fn read_debug(r: &mut Reader, sizecode: usize, string: &dyn Fn(u64) -> Option<String>) -> Result<Vec<String>, String> {
    if r.u8()? != 0 {
        let gap = r.u8()?;
        let intervals = ((sizecode.saturating_sub(1)) >> gap) + 1;
        r.skip(sizecode)?;
        r.skip(intervals * 4)?;
    }
    let mut names = Vec::new();
    if r.u8()? != 0 {
        for _ in 0..r.size()? {
            r.varint()?;
            r.varint()?;
            r.varint()?;
            r.skip(1)?;
        }
        for _ in 0..r.size()? {
            names.push(string(r.varint()?).unwrap_or_default());
        }
    }
    Ok(names)
}

/// Whether an upvalue is (a capture chain ending at) a local of the
/// module's main chunk.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Module,
    Call,
}

/// Check a module's bytecode.
pub fn check(module: &str, bytecode: &[u8]) -> Result<(), Violation> {
    let unreadable = |why: String| Violation {
        module: module.to_string(),
        function: String::new(),
        line: 0,
        rule: Rule::Unreadable(why),
    };
    let (protos, main) = parse(bytecode).map_err(unreadable)?;
    let mut origins: Vec<Option<Vec<Origin>>> = vec![None; protos.len()];
    origins[main] = Some(Vec::new());
    let mut queue = vec![main];
    while let Some(p) = queue.pop() {
        let parent = &protos[p];
        let parent_origins = origins[p].clone().expect("queued protos have origins");
        let code = &parent.code;
        let mut pc = 0;
        while pc < code.len() {
            let insn = code[pc];
            let opcode = (insn & 0xFF) as u8;
            let child = match opcode {
                op::NEWCLOSURE => parent.children.get((insn >> 16) as usize).copied(),
                op::DUPCLOSURE => parent.closures.get(&((insn >> 16) as usize)).copied(),
                _ => None,
            };
            pc += if op::WITH_AUX.contains(&opcode) { 2 } else { 1 };
            let Some(child) = child else { continue };
            // The captures follow the closure instruction, one per upvalue.
            let mut ups = Vec::new();
            while pc < code.len() && (code[pc] & 0xFF) as u8 == op::CAPTURE {
                let (kind, index) = (((code[pc] >> 8) & 0xFF) as u8, ((code[pc] >> 16) & 0xFF) as usize);
                ups.push(if kind == CAPTURE_UPVAL {
                    parent_origins.get(index).copied().unwrap_or(Origin::Call)
                } else if p == main {
                    Origin::Module
                } else {
                    Origin::Call
                });
                pc += 1;
            }
            if child >= protos.len() {
                return Err(unreadable(format!("closure of unknown function {child}")));
            }
            if origins[child].is_none() {
                ups.resize(protos[child].nups as usize, Origin::Call);
                origins[child] = Some(ups);
                queue.push(child);
            }
        }
    }
    for (i, p) in protos.iter().enumerate() {
        let function = p.name.clone().unwrap_or_else(|| if i == main { "(module)".into() } else { "(anonymous)".into() });
        let violation = |rule| Violation { module: module.to_string(), function: function.clone(), line: p.line, rule };
        let mut pc = 0;
        while pc < p.code.len() {
            let insn = p.code[pc];
            let opcode = (insn & 0xFF) as u8;
            match opcode {
                op::SETGLOBAL => return Err(violation(Rule::AssignsGlobal)),
                op::SETUPVAL if i != main => {
                    let index = ((insn >> 16) & 0xFF) as usize;
                    let origin = origins[i].as_ref().and_then(|o| o.get(index).copied());
                    // A function never reached from the module's code can't
                    // run; treat it as module-level to fail closed.
                    if origin != Some(Origin::Call) {
                        let name = p.upvalue_names.get(index).cloned().unwrap_or_else(|| format!("upvalue {index}"));
                        return Err(violation(Rule::AssignsModuleLocal { name }));
                    }
                }
                _ => {}
            }
            pc += if op::WITH_AUX.contains(&opcode) { 2 } else { 1 };
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compile(src: &str) -> Vec<u8> {
        mlua::chunk::Compiler::new().set_debug_level(2).compile(src).unwrap()
    }

    fn verdict(src: &str) -> Result<(), Rule> {
        check("test", &compile(src)).map_err(|v| v.rule)
    }

    #[test]
    fn stateless_modules_pass() {
        let src = r#"
            local LIMIT = 10
            local M = {}
            function M.update(me)
                local n = 0
                local function bump() n += 1 end   -- a call-local: fine
                for _ = 1, LIMIT do bump() end
                return n
            end
            return M
        "#;
        assert_eq!(verdict(src), Ok(()));
    }

    #[test]
    fn module_level_counters_are_rejected() {
        let src = r#"
            local ticks = 0
            return { update = function(me) ticks += 1 end }
        "#;
        assert_eq!(verdict(src), Err(Rule::AssignsModuleLocal { name: "ticks".into() }));
    }

    #[test]
    fn module_locals_written_through_nested_closures_are_rejected() {
        let src = r#"
            local seen = nil
            return { update = function(me)
                local function remember() seen = me end
                remember()
            end }
        "#;
        assert_eq!(verdict(src), Err(Rule::AssignsModuleLocal { name: "seen".into() }));
    }

    #[test]
    fn global_writes_are_rejected() {
        assert_eq!(verdict("function update(me) end"), Err(Rule::AssignsGlobal));
        assert_eq!(verdict("return { update = function() counter = 1 end }"), Err(Rule::AssignsGlobal));
    }

    #[test]
    fn unreadable_bytecode_fails_closed() {
        let mut b = compile("return 1");
        b.truncate(b.len() / 2);
        assert!(matches!(check("test", &b).map_err(|v| v.rule), Err(Rule::Unreadable(_))));
        assert!(matches!(check("test", &[200]).map_err(|v| v.rule), Err(Rule::Unreadable(_))));
    }
}
