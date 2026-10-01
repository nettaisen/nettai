--!strict
-- Spreadr1-3: a bullet from the panel ahead that bursts over the 3x3
-- around what it finds (action.luau); and SpoutCross's charged shot, the
-- same action with its water cannon, fired low, and a splash that hits
-- the panel and the one behind it.
--
-- The Spreaders' records stay the pack's for now (H-Burst's Program
-- Advance recipes name them by number): these are their actions, which
-- registration by number runs (docs/design/content-model-v2.md §5.1, "As
-- built"). SpoutCross's charged shot (weapon 0x0C, still v1) reaches its
-- action the same way, by the number and variant it sets up.

local bullet = require("../../objects/bullet/bullet")
local spreader = require("./action")

-- 16 pixels up (`dword_80ECC5C`'s rows 0, 1 and 3), or on the ground (row 2).
local HIGH, LOW = 0x10_0000, 0

local function spreadr(id: string): Action
    return spreader.action { id = id, gun = spreader.guns.spreader, z = HIGH, bullet = bullet.variants.spreader }
end

return {
    spreadr1 = spreadr("spreadr1/action"),
    spreadr2 = spreadr("spreadr2/action"),
    spreadr3 = spreadr("spreadr3/action"),
    spout_charge = spreader.action {
        gun = spreader.guns.water_cannon,
        z = LOW,
        bullet = bullet.variants.spout,
    },
}
