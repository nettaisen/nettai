--!strict
-- The B+Back special of the patch card dustman
-- (`patch-cards/dustman/back-special`): the fan, TenguCross's wind (the
-- original's instant effect 0x14, `sub_80CD4AC`: a gust down each row
-- toward the navi), without TenguCross's cooldown. The original's weapon
-- routine 0x86 (`sub_801254A`).

local tengu_wind = require("../../navis/megaman/weapons/tengu_wind")

-- `sub_801254A`: its effect is the weapon's `instant`, which the instant
-- chips' action runs.
local function setup(_navi: Object)
    return nil
end

return new.weapon {
    id = "patch-cards/dustman/back-special",
    -- The charge table's row 0x86, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 160, 150, 140 },
    setup = setup,
    -- (The instant effect 0x14: the navi waits 8 ticks after it.)
    instant = tengu_wind.blow,
    instant_waits = true,
}
