--!strict
-- MegaMan's charged shot in BN5 (`megaman/charged-shot`): the original's weapon routine 1 (0x0800F5BC, BN6's
-- `sub_8011A7E`), which sets up BN6's charged shot (@bn6/navis/megaman/weapons/charged-shot, BN5's action 0x1E:
-- the same code but for its muzzle flash's attachment row, 13 for BN6's 5, the same look) with (Attack + 1) * 10
-- damage. Unlike BN6's it has no worn-out rule, and it always draws: on half the draws the shot is the one the
-- navi's charged-shot program names (NaviStats +0x4F, the plain shot when 0), else the charged shot.

local charged_shot = require("@bn6/navis/megaman/weapons/charged-shot/weapon")
local variants = require("../../../objects/projectile/variants")

local bn5 = {}

-- 0x0800F5BC.
function bn5.setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = (stats.attack + 1) * 10
    local shot = variants.charged
    if bit32.band(battle.rng_positive(), 0x10) ~= 0 then
        shot = stats.charge_shot_kind or variants.plain
    end
    local s = navi:action_state(charged_shot.action) :: { projectile: ProjectileVariant? }
    s.projectile = shot
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return charged_shot.action
end

bn5.weapon = define.weapon {
    id = "megaman/charged-shot",
    -- The charge table's row 1 (0x0801CA6C, BN6's values); Charge 5 reads on into row 2's first entry.
    charge_ticks = { 100, 90, 80, 70, 60, 180 },
    setup = bn5.setup,
}

return bn5
