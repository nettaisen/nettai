--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- records of the trap chips (dimming chip subtype 20: AntiNavi, AntiDmg,
-- AntiSwrd, AntiRecv and BodyGrd, which the ruleset names by number;
-- BodyGrd is a Program Advance of the first three) run their chips'
-- `dimming` hooks, picked by the first chip parameter as the original's
-- controller picks its trap's object (`off_80E3574`). ElemTrap is a
-- definition (chips/elemtrap) and doesn't come here; its row is kept for
-- a record that names it. It goes when the ruleset names these chips by
-- trait (phase C, step 10).

local antidmg = require("../antidmg/chip")
local antinavi = require("../antinavi/chip")
local antirecv = require("../antirecv/chip")
local antiswrd = require("../antiswrd/chip")
local bodygrd = require("../bodygrd/chip")
local elemtrap = require("../elemtrap/chip")

local HOOKS: { [number]: (user: Object, spec: DimmingChipSpec) -> Object? } = {
    [0] = elemtrap.dimming,
    [1] = antirecv.dimming,
    [2] = antinavi.dimming,
    [3] = antidmg.dimming,
    [4] = antiswrd.dimming,
    [5] = bodygrd.dimming,
}

local function dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local row = spec.params[1]
    local hook = HOOKS[row]
    if not hook then
        error(string.format("trap chip Param1 %d reads past sub_80E3560's table (off_80E3574)", row))
    end
    return hook(user, spec)
end

return { dimming_chip = dimming_chip }
