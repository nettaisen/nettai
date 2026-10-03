--!strict
-- MegaMan's buster (`megaman/buster`): the original's weapon routine 0
-- (`sub_8011A26`), which sets up action 0x11 with the buster's damage, and
-- action 0x11 (`sub_80EB436`), the shot (`buster.shot`). The NaviCust's
-- buster bugs may make a shot a blank or a charged shot instead (one RNG
-- draw every shot), and its buster-shot program the shot's kind (another
-- draw). The routine numbers that alias it (0x2E, 0x2F, 0x3E, ...) are
-- this weapon's in compat (weapons.toml). See docs/engine/objects-and-player.md
-- §B6.
--
-- Action 0x11 (@common/megaman/buster, with BN6's buster parts, projectile,
-- attachment and throw) raises the buster; on its second tick the projectile flies
-- from the panel in front (the spread adds shots a row up and down; a
-- throw, `throw_absorbed`'s, throws the obstacle the navi absorbed
-- instead), with a muzzle flash in the navi's first related slot; after
-- five ticks, the recovery, which a move may cut short.

local attachment = require("../../../../objects/attachment/attachment")
local blank_shot = require("../blank-shot/weapon")
local common = require("@common/megaman/buster")
local charged_shot = require("../charged-shot/weapon")
local flying_shot = require("../../../../objects/flying-shot/flying_shot")
local lib = require("../../../../lib/buster")
local projectile = require("../../../../lib/projectile")
local variants = require("../../../../objects/projectile/variants")

-- (The recovery after the shot is the navi's `attack_count`, AIAttackVars+0x12:
-- a throw doesn't write it, so it waits the last shot's, whatever ran in
-- between.)
export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
    -- What it fires (the original's attack variant 0, 1 or 2): the shot,
    -- the spread (two more shots a row up and down), or the obstacle the
    -- navi absorbed.
    mode: "shot" | "spread" | "throw",
    -- The projectile (the original's first attack parameter, a row of the
    -- projectile's table), which a throw doesn't read.
    projectile: ProjectileVariant?,
}

local buster = {}

-- The height a thrown obstacle flies at, 16.16.
local THROW_Z = 0x0C_0000

-- The buster-shot programs that come on 1 draw in 8 (the others on 1 in
-- 2): the original's rows from 0x1E on.
local RARE: { [ProjectileVariant]: boolean } = {
    [variants.pushing] = true,
    [variants.shoving] = true,
    [variants.charged_pulling] = true,
    [variants.charged_bubbling] = true,
    [variants.charged_road] = true,
    [variants.charged_hp_bug_marked] = true,
    [variants.charged_road_back] = true,
    [variants.charged_grass] = true,
    [variants.charged_freezing] = true,
    [variants.charged_ice] = true,
}

-- `sub_8013D5E`: what this shot is (@common/megaman/buster).
buster.pick = common.pick

-- `sub_8011A26`.
function buster.setup(navi: Object): Action
    local shot = buster.pick(navi)
    if shot == "blank" then
        return blank_shot.setup(navi)
    elseif shot == "charged" then
        return charged_shot.setup(navi)
    end
    navi.attack_damage = navi:buster_damage()
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    -- The buster-shot program (a projectile variant the navi's stats
    -- name), on a lucky draw.
    local program = battle.navi(navi.alliance).buster_shot
    if program then
        local mask = if RARE[program] then 7 else 1
        if bit32.band(battle.rng_positive(), mask) ~= 0 then
            program = nil
        end
    end
    local s = navi:action_state(buster.shot) :: State
    s.mode = "shot"
    s.projectile = program or variants.plain
    return buster.shot
end

-- The shot as a throw of the obstacle the navi absorbed: what
-- `throw_absorbed`'s setups start instead of a shot.
function buster.throw(navi: Object): Action
    local s = navi:action_state(buster.shot) :: State
    s.mode = "throw"
    return buster.shot
end

-- `sub_80EB450`'s throw: the last obstacle absorbed (what the throw's
-- setup took from the absorbed list into the attack: the original's marker
-- word) flies from the panel in front (`sub_80C6248`).
local function throw(me: Object)
    local x, y = field.center(me.panel_x + me.facing, me.panel_y)
    flying_shot.launch(me, Vec3.new(x, y, THROW_Z), {
        variant = flying_shot.variants.thrown,
        damage = common.damage_word(me),
        obstacle = me.thrown_look,
        obstacle_anim = me.thrown_anim,
    })
end

-- The shot; a thrown obstacle (`throw_absorbed`'s) is one too. (The
-- sounds: the shot, and a throw's besides, the junk shot's.)
buster.shot = common.shot_action {
    id = "bn6:megaman/buster/shot",
    buster = lib,
    projectiles = projectile,
    attachment = attachment.kind,
    shot_sound = asset.sound("bn6:buster-shot"),
    plain = variants.plain,
    throw_sound = asset.sound("bn6:junk-shot"),
    throw = throw,
}

buster.weapon = define.weapon {
    id = "bn6:megaman/buster",
    -- The buster isn't charged (the charge table's row 0).
    charge_ticks = { 0, 0, 0, 0, 0 },
    setup = buster.setup,
    -- (`off_80117D4`'s other entries that are `sub_8011A26` read other
    -- rows of the charge table, so they are weapons of their own with this
    -- setup: navis/megaman/weapons/buster-2e and the rest.)
}

return buster
