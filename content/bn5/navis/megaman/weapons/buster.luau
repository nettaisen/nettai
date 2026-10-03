--!strict
-- MegaMan's buster in BN5 (`megaman/buster`): the original's weapon routine 0 (0x0800F522, BN6's `sub_8011A26`),
-- which sets up BN6's buster shot (@bn6/navis/megaman/weapons/buster, BN5's action 0x11: the same code but for
-- the muzzle flash's attachment row and the absorbed-obstacle throw, which BN5 hasn't). The NaviCust's buster
-- bugs may make a shot a blank or a charged shot instead (BN6's pick, `sub_8013D5E`: the same code). Unlike
-- BN6's, the damage is Attack plus the navi's buster bonus (0x0800F5AC by navi: no worn-out rule, no form's
-- bonus, no cap), and it always draws: on half the draws the shot is the one the navi's buster-shot program
-- names (NaviStats +0x4D, the plain shot when 0).
--
-- (Not here: BN5's MegaMan in his base form with NaviStats +0x4C set fires a spread of projectile row 18;
-- bn5-compat refuses such stats until it is.)

local blank_shot = require("@bn6/navis/megaman/weapons/blank-shot/weapon")
local buster = require("@bn6/navis/megaman/weapons/buster/weapon")
local charged_shot = require("./charged-shot")
local variants = require("../../../objects/projectile/variants")

local bn5 = {}

-- 0x0800F522.
function bn5.setup(navi: Object): Action
    local shot = buster.pick(navi)
    if shot == "blank" then
        return blank_shot.setup(navi)
    elseif shot == "charged" then
        return charged_shot.setup(navi)
    end
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = stats.attack + stats.navi.buster_bonus
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    local program = variants.plain
    if bit32.band(battle.rng_positive(), 4) ~= 0 then
        program = stats.buster_shot or variants.plain
    end
    local s = navi:action_state(buster.shot) :: { mode: string, projectile: ProjectileVariant? }
    s.mode = "shot"
    s.projectile = program
    return buster.shot
end

bn5.weapon = define.weapon {
    id = "bn5:megaman/buster",
    -- The buster isn't charged (the charge table's row 0).
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = bn5.setup,
}

return bn5
