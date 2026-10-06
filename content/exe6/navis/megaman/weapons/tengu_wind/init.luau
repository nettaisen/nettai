--!strict
-- TenguCross's B+Back special (`megaman/tengu-wind`): the original's
-- weapon routine 0x10 (`sub_8011CA8`), which names the instant chips'
-- action with an effect no chip has (`sub_80CD4AC`, the original's instant
-- effect 0x14) and a 40-tick cooldown before the next B+Back. The effect
-- blows a gust (objects/gust) down each row from the far column toward the
-- navi's own side; the navi then waits 8 ticks (the instant chips' action's
-- wait for this effect).

local common = require("@exelib/gust")
local gust = require("../../../objects/gust")

local tengu_wind = {}

local COOLDOWN = 0x28

local SOUND = asset.sound("dustbrk")

-- `sub_8011CA8`: the cooldown; its effect is the weapon's `instant`, which
-- the instant chips' action runs.
local function setup(navi: Object)
    navi.attack_lockout = COOLDOWN
    return nil
end

-- `sub_80CD4AC` (instant effect 0x14; the patch card dustman's B+Back
-- too): a gust in each row, pulling (@exelib/gust/gust's `blow_rows`).
function tengu_wind.blow(user: Object, _spec: InstantChipSpec)
    common.blow_rows(gust.kind, user, SOUND)
end

tengu_wind.weapon = define.weapon {
    id = "megaman/tengu-wind",
    -- The charge table's row 0x10, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 0, 0, 0, 0, 0, 60, 60, 60 },
    setup = setup,
    -- (The original's instant effect 0x14, which no chip has: the navi
    -- waits 8 ticks after it.)
    instant = tengu_wind.blow,
    instant_waits = true,
}

return tengu_wind
