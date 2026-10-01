--!strict
-- MegaMan's charged shot (`megaman/charged-shot`): the original's weapon
-- routine 1 (`sub_8011A7E`), which sets up action 0x16 with (Attack + 1) *
-- 10 damage (10 when worn out), and action 0x16 (`sub_80EBE00`), the shot.
-- The ruleset also starts the shot without the setup (the role
-- `forced_charged_shot`, rules/roles.luau). The NaviCust's charged-shot program
-- changes the shot's kind on a lucky draw. See
-- docs/engine/objects-and-player.md §B7.
--
-- Action 0x16 waits 5 ticks, raises the buster, and on the next tick the
-- projectile flies from the panel in front with a muzzle flash in the
-- navi's first related slot; after five ticks, the recovery, which a move
-- may cut short.

local attachment = require("../../../../objects/attachment/attachment")
local lib = require("../../../../lib/buster")
local projectile = require("../../../../lib/projectile")
local variants = require("../../../../objects/projectile/variants")

-- (The recovery after the shot is the navi's `recovery`, AIAttackVars+0x12,
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

-- The phases, on the attack's step.
local WAIT, SHOOT, RECOVER = 0x0, 0x4, 0x8

-- The wait before the buster comes up, in ticks.
local WAIT_TICKS = 5
-- The shot's sound and height (16.16).
local SHOT_SOUND, SHOT_Z = asset.sound("buster-shot"), 0x18_0000

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

-- `sub_80EBE54`: raise the buster; on the second tick the shot (its
-- damage without the hit parameter) and the flash; after five ticks, the
-- recovery. (Started without its setup, by the role `forced_charged_shot`,
-- it fires what the attack's first parameter last held: here the last
-- charged shot's projectile, or the table's first row.)
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        lib.raise_arm(me)
        me.step_init = 4
        me:set_status("using_action", true)
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SHOT_SOUND)
        local variant = s.projectile or variants.plain
        me.recovery = projectile.fire(me, { variant = variant, damage = me.attack_damage, z = SHOT_Z })
        attachment.attach(me, lib.flash, "related")
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EBE20`: wait; then the shot starts at once. (Its entry also
-- calls `nullsub_12` with the panel in front.)
local function wait(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = WAIT_TICKS
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left <= 0 then
        me.step = SHOOT
        me.step_init = 0
        shoot(me, s)
    end
end

-- `sub_80EBEB2`: the recovery; a move may cut it short, even on the tick
-- it ends (there is no return after the exit).
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = me.recovery
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
    lib.move_cancel(me)
end

-- `sub_80EBE00`.
local function update(me: Object, s: State)
    local step = me.step
    if step == WAIT then
        wait(me, s)
    elseif step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("charged shot phase %#x reads past its phase table", step))
    end
end

charged_shot.action = define.action {
    id = "megaman/charged-shot/action",
    state = { timer = "u16", projectile = "record:projectile-variant" },
    update = update,
}

charged_shot.weapon = define.weapon {
    id = "megaman/charged-shot",
    name = "Charged shot",
    -- The charge table's row 1; Charge 5 reads on into row 2's first
    -- entry, as the original's table does.
    charge_ticks = { 100, 90, 80, 70, 60, 180 },
    setup = charged_shot.setup,
}

return charged_shot
