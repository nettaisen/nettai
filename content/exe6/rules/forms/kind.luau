--!strict
-- EXE6's kinds of form: the forms system's extension `kind` (system.luau),
-- which every EXE6 form but the base form names: a Cross, Beast Out ("beast"),
-- a Cross in Beast Out ("cross_beast") or Beast Over. What EXE6's rules and
-- chips ask of a side's form by it (the original's tests of the form's
-- number). A form of another game has no kind: it is none of these.

local kind = {}

-- `kind` is a Beast form's: Beast Out, with or without a Cross, or Beast
-- Over (forms 0x0B to 0x18).
function kind.is_beast(k: string?): boolean
    return k == "beast" or k == "cross_beast" or k == "beast_over"
end

-- The side's form is a Beast form.
function kind.beast(side: number): boolean
    return kind.is_beast(battle.navi(side).form.kind)
end

-- The side's form is Beast Over (forms 0x17 and 0x18).
function kind.beast_over(side: number): boolean
    return battle.navi(side).form.kind == "beast_over"
end

return kind
