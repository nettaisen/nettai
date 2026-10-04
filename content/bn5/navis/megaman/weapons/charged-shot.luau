--!strict
-- MegaMan's charged shot in BN5 (`megaman/charged-shot`): the original's weapon routine 1 (0x0800F5BC, BN6's
-- `sub_8011A7E`), which sets up the shared charged shot (@exelib/megaman/charged_shot, BN6's action 0x16, BN5's
-- action 0x1E: the same code but for its muzzle flash's attachment row, 13 for BN6's 5, the same look, and its wait,
-- 0x080EC9F0, which only the battle flag 0x40 mode makes, marking the panel in front for 10 ticks) with BN5's
-- buster parts, projectile and attachment, and (Attack + 1) * 10 damage. (Started without its setup, by the role
-- `forced_charged_shot`, it fires the last charged shot's projectile, or BN6's plain shot, as BN6's action did: BN5's
-- table's first row is BN5's plain shot, which the port has yet to check.) Unlike BN6's it has no worn-out rule, and it always draws: on half the draws the shot is the one the
-- navi's charged-shot program names (NaviStats +0x4F, the plain shot when 0), else the charged shot.

local attachment = require("../../../objects/attachment/attachment")
local bn6_variants = require("@bn6/objects/projectile/variants")
local buster = require("../../../lib/buster")
local common = require("@exelib/megaman/charged_shot")
local marker = require("../../../lib/navi-chips/marker")
local projectile = require("../../../lib/projectile")
local variants = require("../../../objects/projectile/variants")

local bn5 = {}

-- The shot (BN5's action 0x1E).
bn5.action = common.action {
    id = "bn5:megaman/charged-shot/action",
    buster = buster,
    projectiles = projectile,
    attachment = attachment.kind,
    shot_sound = asset.sound("bn6:buster-shot"),
    plain = bn6_variants.plain,
    -- 0x080EC9F0: in the battle flag 0x40 mode, the wait, its panel in front marked (0x080E9FD2: style 8, 10
    -- ticks); else none.
    waits = function(me: Object): boolean
        if not battle.per_player_gauges() then
            return false
        end
        local front = if bit32.bxor(me.alliance, me.flip) == 0 then 1 else -1
        marker.spawn(me, me.panel_x + front, me.panel_y, 10, 8, true)
        return true
    end,
}
local charged_shot = { action = bn5.action }

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

-- 0x0800FDD0: the patch cards' ChrgS (B↓ChrgS), the charged shot without the draw: (Attack + 1) * 10 damage, always
-- the shot the navi's charged-shot program names (NaviStats +0x4F; the charged shot, row 6, when 0).
function bn5.program_setup(navi: Object): Action
    local stats = battle.navi(navi.alliance)
    navi.attack_damage = (stats.attack + 1) * 10
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    local s = navi:action_state(charged_shot.action) :: { projectile: ProjectileVariant? }
    -- (The program's 0 is the projectile table's first row, the plain shot: none, here.)
    local program = stats.charge_shot_kind
    s.projectile = if program == nil or program == variants.plain then variants.charged else program
    return charged_shot.action
end

bn5.program = define.weapon {
    id = "bn5:megaman/program-charged-shot",
    -- The charge table's row 0x63, by Charge 0 to 5: past 4 it reads on into row 0x64's first entry.
    charge_ticks = { 100, 90, 80, 70, 60, 80 },
    setup = bn5.program_setup,
}

bn5.weapon = define.weapon {
    id = "bn5:megaman/charged-shot",
    -- The charge table's row 1 (0x0801CA6C, BN6's values); Charge 5 reads on into row 2's first entry.
    charge_ticks = { 100, 90, 80, 70, 60, 180 },
    setup = bn5.setup,
}

return bn5
