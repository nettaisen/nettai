--!strict
-- MegaMan's buster in BN5 (`megaman/buster`): the original's weapon routine 0 (0x0800F522, BN6's `sub_8011A26`),
-- which sets up the shared buster shot (@exelib/megaman/buster, BN6's action 0x11, BN5's 0x11: the same code but
-- for the muzzle flash's attachment row and the absorbed-obstacle throw, which BN5 hasn't) with BN5's buster parts,
-- projectile and attachment; its sound BN6's. The NaviCust's buster bugs may make a shot a blank (the shared blank
-- shot, @exelib/megaman/blank_shot: BN5's action 0x33, its click BN6's) or a charged shot instead (BN6's pick,
-- `sub_8013D5E`: the same code). Unlike
-- BN6's, the damage is Attack plus the navi's buster bonus (0x0800F5AC by navi: no worn-out rule, no form's
-- bonus, no cap), and it always draws: on half the draws the shot is the one the navi's buster-shot program
-- names (NaviStats +0x4D, the plain shot when 0).
--
-- (Not here: BN5's MegaMan in his base form with NaviStats +0x4C set fires a spread of projectile row 18;
-- bn5-compat refuses such stats until it is.)

local attachment = require("../../../objects/attachment/attachment")
local blank = require("@exelib/megaman/blank_shot")
local charged_shot = require("./charged-shot")
local common = require("@exelib/megaman/buster")
local lib = require("../../../lib/buster")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")

local bn5 = {}

-- The shot (BN5's action 0x11), which throws no absorbed obstacle (BN5 absorbs none).
local buster = {
    pick = common.pick,
    shot = common.shot_action {
        id = "bn5:megaman/buster/shot",
        buster = lib,
        projectiles = projectile,
        attachment = attachment.kind,
        shot_sound = asset.sound("bn6:buster-shot"),
        plain = variants.plain,
        throw_sound = asset.sound("bn6:junk-shot"),
    },
}

-- The blank shot (BN5's action 0x33).
local blank_shot = blank.make { id = "bn5:megaman/blank-shot/action", buster = lib, click = asset.sound("bn6:gundels1") }

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
