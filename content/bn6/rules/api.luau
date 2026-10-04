--!strict
-- BN6's API (docs/design/rules-in-luau.md, As built S8): what BN6's chips and
-- objects ask of BN6's rules, which keep it in their systems. Content
-- outside rules/ asks it here, never a system's state itself. Each answers
-- as BN6 would without the system it reads, when the side's ruleset (a
-- variant) lacks it.

local dark_chips = require("./dark-chips/system")
local kind = require("./forms/kind")

local bn6 = {}

-- The side's bug frags (`sub_800F4A8`: the dark-chips system's state; 0
-- without the system).
function bn6.bug_frags(side: number): number
    local s = system.state_of(side, dark_chips) :: { bug_frags: number }?
    return if s then s.bug_frags else 0
end

-- `sub_800F4B2`: the side spends `n` bug frags if it has them (the
-- original's callers check first). Whether it did: never without the
-- dark-chips system.
function bn6.spend_bug_frags(side: number, n: number): boolean
    local s = system.state_of(side, dark_chips) :: { bug_frags: number }?
    if s == nil or s.bug_frags < n then
        return false
    end
    s.bug_frags -= n
    return true
end

-- The level of the navi code the side's save received (0 to 14), or nil
-- (event flag 0x163 clear): the engine's, which a link navi's chip bonus
-- reads too.
function bn6.navi_level(side: number): number?
    return battle.navi_level(side)
end

-- The side's form's kind (the forms system's extension: rules/forms/kind):
-- a Beast form's; Beast Over; a kind that is a Beast form's.
bn6.beast = kind.beast
bn6.beast_over = kind.beast_over
bn6.is_beast = kind.is_beast

return bn6
