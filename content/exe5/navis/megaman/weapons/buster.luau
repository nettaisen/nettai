--!strict
-- MegaMan's buster in EXE5 (`megaman/buster`): the original's weapon routine 0 (0x0800F522, EXE6's `sub_8011A26`), which
-- sets up the shared buster shot (@exelib/megaman/buster, EXE6's action 0x11, EXE5's 0x11: the same code but for the
-- muzzle flash's attachment row and the absorbed-obstacle throw, which EXE5 hasn't) with EXE5's buster parts, projectile
-- and attachment; its sound EXE6's number. The NaviCust's buster bugs may make a shot a blank (the shared blank shot,
-- @exelib/megaman/blank_shot: EXE5's action 0x33, its click EXE6's number) or a charged shot instead (EXE6's pick,
-- `sub_8013D5E`: the same code). Unlike EXE6's, the damage is Attack plus the navi's buster bonus (0x0800F5AC by navi:
-- no worn-out rule, no form's bonus, no cap), and it always draws: on half the draws the shot is the one the navi's
-- buster-shot program
-- names (NaviStats +0x4D, the plain shot when 0). MegaMan in his base form in Hub Style (NaviStats +0x4C, patch card
-- 111's) fires the spread instead (the shot's variant 1: a row up and down too) of Hub Style's shot (projectile row
-- 0x12), with its own flash (the attachment table's row 0x39, 0c-59: 0x080EBA30).
--
-- The patch cards' TriBustr (B button routine 0x65, 0x0800FE12) is this routine with its own charge row.

local attachment = require("../../../objects/attachment")
local blank = require("@exelib/megaman/blank_shot")
local charged_shot = require("./charged_shot")
local common = require("@exelib/megaman/buster")
local lib = require("../../../lib/buster")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")

local exe5 = {}

-- EXE5's MegaMan (navi 0): his navi requires this module, so by his key.
local MEGAMAN = "megaman"

-- The shot (EXE5's action 0x11), which throws no absorbed obstacle (EXE5 absorbs none).
local buster = {
    pick = common.pick,
    shot = common.shot_action {
        id = "megaman/buster/shot",
        buster = lib,
        projectiles = projectile,
        attachment = attachment.kind,
        shot_sound = asset.sound("buster-shot"),
        plain = variants.plain,
        throw_sound = asset.sound("junk-shot"),
        spread_flash = attachment.look { sprite = asset.sprite("spread-flash"), palette = 0, attach_point = 0 },
    },
}

-- The blank shot (EXE5's action 0x33).
local blank_shot = blank.make { id = "megaman/blank-shot/action", buster = lib, click = asset.sound("gundels1") }

-- 0x0800F522.
function exe5.setup(navi: Object): Action
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
    local mode = "shot"
    -- MegaMan (navi 0, NaviStats +0x29) in his base form (+0x2C 0) in Hub Style (+0x4C): the spread.
    if stats.navi.id == MEGAMAN and stats.form.base and stats.hub_style ~= 0 then
        program, mode = variants.hub, "spread"
    end
    local s = navi:action_state(buster.shot) :: { mode: string, projectile: ProjectileVariant? }
    s.mode = mode
    s.projectile = program
    return buster.shot
end

exe5.weapon = define.weapon {
    id = "megaman/buster",
    -- The buster isn't charged (the charge table's row 0).
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = exe5.setup,
}

-- 0x0800FE12: the patch cards' TriBustr (a B button), the buster's routine.
exe5.tri_buster = define.weapon {
    id = "megaman/tri-buster",
    -- The charge table's row 0x65, by Charge 0 to 5: past 4 it reads on into row 0x66's first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 120 },
    setup = exe5.setup,
}

return exe5
