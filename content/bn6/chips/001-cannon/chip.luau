--!strict
-- Registration by number (docs/design/content-model-v2.md §5.1, "As
-- built"): a record that names the original's action 0x14 runs this
-- module, which runs the chip's own action (chips/cannon, chips/gigacan)
-- picked by the subtype as the original's tables (`byte_80EBD2C`,
-- `byte_80EBD34`) pick the cannon and the shot. Records reach it where
-- something names a chip by number: the cannons and GigaCans (Program
-- Advances), the Cross special's cannons (the ruleset's table of numbers),
-- and the nameless copies of Cannon's record. It goes when those are
-- definitions or roles (step 5, phase C).

local cannon = require("../../lib/cannon")
local cannons = require("../cannon/chips")
local gigacans = require("../gigacan/chips")

local legacy = {
    state = cannon.STATE,
}

-- The chips' actions by subtype. Subtype 3 (the cannon row 0x1E with the
-- plain cannon shot) has no chip.
local BY_SUBTYPE: { [number]: Action } = {
    [0] = cannons.cannon,
    [1] = cannons.hicannon,
    [2] = cannons.m_cannon,
    [4] = gigacans.gigacan1,
    [5] = gigacans.gigacan2,
    [6] = gigacans.gigacan3,
}

function legacy.update(me: Object, s: any)
    local v = me.variant
    local a = BY_SUBTYPE[v]
    if not a then
        error(string.format("cannon subtype %#x: no chip has it (byte_80EBD2C)", v))
    end
    a.update(me, s)
end

return legacy
