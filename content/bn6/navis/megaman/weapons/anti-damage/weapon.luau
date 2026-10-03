--!strict
-- The AntiDmg program's B+Back special (`megaman/anti-damage`): the
-- original's weapon routine 0x3D (`sub_80121BC`) and its stance, action
-- 0x5A (`sub_80EFE7C`), which BN5's shares (@common/navicust/anti-damage).
-- For 31 ticks the navi's trap is armed; a caught hit turns the stance into
-- the counter with the stance's 50 damage. The navi's chips then wait 70
-- ticks.

local shared = require("@common/navicust/anti-damage")

return shared.make {
    weapon = "bn6:megaman/anti-damage",
    action = "bn6:megaman/anti-damage/action",
    -- (A B+Back special, which nothing charges: the charge table's row
    -- 0x3D by Charge 0 to 7, reading on into the next routine's row.)
    charge_ticks = { 0, 0, 0, 0, 0, 0, 0, 0 },
}
