--!strict
-- MegaMan's charged shot (`megaman/charged-shot`): the original's weapon
-- routine 1 (`sub_8011A7E`), which sets up action 0x16 with (Attack + 1) *
-- 10 damage (10 when worn out), and action 0x16 (`sub_80EBE00`), the shot.
-- The ruleset also starts the shot without the setup (the role
-- `forced_charged_shot`, rules/roles.luau). The NaviCust's charged-shot program
-- changes the shot's kind on a lucky draw. See
-- docs/engine/objects-and-player.md §B7.
--
-- Action 0x16 (@exelib/megaman/charged_shot, with BN6's buster parts,
-- projectile and attachment) waits 5 ticks, raises the buster, and on the next tick the
-- projectile flies from the panel in front with a muzzle flash in the
-- navi's first related slot; after five ticks, the recovery, which a move
-- may cut short.

local attachment = require("../../../../objects/attachment/attachment")
local common = require("@exelib/megaman/charged_shot")
local lib = require("../../../../lib/buster")
local projectile = require("../../../../lib/projectile")
local variants = require("../../../../objects/projectile/variants")

-- (The recovery after the shot is the navi's `attack_count`, AIAttackVars+0x12,
-- which a thrown obstacle waits again.)
export type State = {
    -- Ticks left of the wait, ticks into the shot, then ticks of recovery
    -- left (AIAttackVars+0x10).
    timer: number,
    -- The projectile (the original's first attack parameter, a row of the
    -- projectile's table).
    projectile: ProjectileVariant?,
}

local charged_shot = {}

-- The charged-shot programs that come on 1 draw in 8 (the others on 1 in
-- 2): the original's rows 9 and 0x23.
local RARE: { [ProjectileVariant]: boolean } = {
    [variants.attack_90] = true,
    [variants.charged_hp_bug_marked] = true,
}

-- `sub_8011A7E`.
function charged_shot.setup(navi: Object): Action
    local side = navi.alliance
    local stats = battle.navi(side)
    local base = stats.attack + 1
    if battle.emotion(side) == "worn_out" then
        base = 1
    end
    navi.attack_damage = base * 10
    -- The projectile: the charged-shot program's (a projectile variant the
    -- navi's stats name) on a lucky draw, else the plain charged shot. (A
    -- program that is the plain charged shot draws nothing.)
    local program = stats.charge_shot_kind
    local shot = variants.charged
    if program and program ~= variants.charged then
        local mask = if RARE[program] then 7 else 1
        if bit32.band(battle.rng_positive(), mask) == 0 then
            shot = program
        end
    end
    local s = navi:action_state(charged_shot.action) :: State
    s.projectile = shot
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return charged_shot.action
end

charged_shot.action = common.action {
    id = "bn6:megaman/charged-shot/action",
    buster = lib,
    projectiles = projectile,
    attachment = attachment.kind,
    shot_sound = asset.sound("bn6:buster-shot"),
    plain = variants.plain,
}

charged_shot.weapon = define.weapon {
    id = "bn6:megaman/charged-shot",
    -- The charge table's row 1; Charge 5 reads on into row 2's first
    -- entry, as the original's table does.
    charge_ticks = { 100, 90, 80, 70, 60, 180 },
    setup = charged_shot.setup,
}

return charged_shot
