--!strict
-- EXE6's API (docs/design/rules-in-luau.md, As built S8): what EXE6's chips and
-- objects ask of EXE6's rules, which keep it in their systems. Content
-- outside rules/ asks it here, never a system's state itself. Each answers
-- as EXE6 would without the system it reads, when the side's ruleset (a
-- variant) lacks it.

local dark_chips = require("./dark_chips")
local kind = require("./forms/kind")
local save = require("./save")

local exe6 = {}

-- The side's bug frags (`sub_800F4A8`: the dark-chips system's state; 0
-- without the system).
function exe6.bug_frags(side: number): number
    local s = system.state_of(side, dark_chips) :: { bug_frags: number }?
    return if s then s.bug_frags else 0
end

-- `sub_800F4B2`: the side spends `n` bug frags if it has them (the
-- original's callers check first). Whether it did: never without the
-- dark-chips system.
function exe6.spend_bug_frags(side: number, n: number): boolean
    local s = system.state_of(side, dark_chips) :: { bug_frags: number }?
    if s == nil or s.bug_frags < n then
        return false
    end
    s.bug_frags -= n
    return true
end

-- The level of the navi code the side's save received (0 to 14), or nil
-- (event flag 0x163 clear): its save system's `level` (rules/save), which
-- the engine knows nothing of. What a level gives a link navi in battle is
-- rules/by_level's.
function exe6.navi_level(side: number): number?
    local s = system.setup_of(side, save) :: { level: number? }?
    return if s then s.level else nil
end

-- The side's form's kind (the forms system's extension: rules/forms/kind):
-- a Beast form's; Beast Over; a kind that is a Beast form's.
exe6.beast = kind.beast
exe6.beast_over = kind.beast_over
exe6.is_beast = kind.is_beast

return exe6
