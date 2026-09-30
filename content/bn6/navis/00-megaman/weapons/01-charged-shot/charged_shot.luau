--!strict
-- MegaMan's charged shot: weapon routine 1 (`sub_8011A7E`), which sets up
-- action 0x16 with (Attack + 1) * 10 damage (10 when worn out), and action
-- 0x16 (`sub_80EBE00`), the shot. The NaviCust's charged-shot program
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

export type State = {
    -- Ticks left of the wait, ticks into the shot, then ticks of recovery
    -- left (AIAttackVars+0x10).
    timer: number,
    -- The recovery after the shot (AIAttackVars+0x12).
    recovery: number,
}

local charged_shot = {
    state = { timer = "u16", recovery = "u16" },
}

local ACTION = 0x16

-- The phases, on the attack's step.
local WAIT, SHOOT, RECOVER = 0x0, 0x4, 0x8

-- The wait before the buster comes up, in ticks.
local WAIT_TICKS = 5
-- The shot's sound and height (16.16), and the muzzle flash (attachment
-- 5).
local SHOT_SOUND, SHOT_Z, FLASH = 0x6A, 0x18_0000, 5

-- The plain charged shot's kind.
local PLAIN = 6

-- `sub_8011A7E`.
function charged_shot.setup(navi: Object): number
    local side = navi.alliance
    local stats = battle.navi(side)
    local base = stats.attack + 1
    if battle.emotion(side) == "worn_out" then
        base = 1
    end
    navi.attack_damage = base * 10
    -- The kind: the program's on 1 draw in 8 (kinds 9 and 0x23) or 1 in 2.
    local kind = stats.charge_shot_kind
    if kind == 0 then
        kind = PLAIN
    elseif kind ~= PLAIN then
        local mask = if kind == 9 or kind == 0x23 then 7 else 1
        if bit32.band(battle.rng_positive(), mask) ~= 0 then
            kind = PLAIN
        end
    end
    navi:set_attack_param(1, kind)
    for i = 2, 4 do
        navi:set_attack_param(i, 0)
    end
    navi.attack_element = 0
    navi.variant = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_lockout = 0
    return ACTION
end

-- `sub_80EBE54`: raise the buster; on the second tick the shot (its kind
-- the attack's first parameter, its damage without the hit parameter) and
-- the flash; after five ticks, the recovery.
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
        s.recovery = projectile.fire(me, { kind = me:attack_param(1), damage = me.attack_damage, z = SHOT_Z })
        attachment.spawn(me, { kind = FLASH }, "related")
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
        s.timer = s.recovery
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
function charged_shot.update(me: Object, s: State)
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

return charged_shot
