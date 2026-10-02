--!strict
-- The patch cards' "ChrgS" charged shot (`patch-cards/chrgs`, the original's
-- weapon routine 0x8D, `sub_80125D0`): nine cards give it, as a bug. It is
-- MegaMan's charged shot (action 0x16), but with (Attack + 1) * 10 damage
-- however worn out the navi is, and its charged-shot program on every shot
-- (the plain charged shot without one), where the plain charged shot's
-- program comes on a lucky draw.

local charged_shot = require("../../navis/megaman/weapons/charged-shot/weapon")
local variants = require("../../objects/projectile/variants")

-- `sub_80125D0`.
local function setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = (stats.attack + 1) * 10
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    local s = navi:action_state(charged_shot.action) :: { projectile: ProjectileVariant? }
    s.projectile = stats.charge_shot_kind or variants.charged
    return charged_shot.action
end

return define.weapon {
    id = "patch-cards/chrgs",
    name = "ChrgS",
    -- The charge table's row 0x8D, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 100, 90, 80, 70, 60, 60, 55, 50 },
    setup = setup,
}
