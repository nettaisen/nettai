--!strict
-- MegaMan's charged shot, as BN5 and BN6 share it (BN6's action 0x16, `sub_80EBE00`; BN5's 0x1E, the same code but
-- for its muzzle flash's attachment row), of a game's `MegaManShotLook` (`charged_shot.action`): it waits 5 ticks,
-- raises the buster, and on the next tick the projectile flies from the panel in front with a muzzle flash in the
-- navi's first related slot; after five ticks, the recovery, which a move may cut short. See
-- docs/engine/objects-and-player.md §B7.

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

local attachment = require("../attachment/attachment")

local charged_shot = {}

-- The phases, on the attack's step.
local WAIT, SHOOT, RECOVER = 0x0, 0x4, 0x8

-- The wait before the buster comes up, in ticks.
local WAIT_TICKS = 5
-- The shot's height (16.16).
local SHOT_Z = 0x18_0000

-- `sub_80EBE54`: raise the buster; on the second tick the shot (its
-- damage without the hit parameter) and the flash; after five ticks, the
-- recovery. (Started without its setup, by the role `forced_charged_shot`,
-- it fires what the attack's first parameter last held: here the last
-- charged shot's projectile, or the table's first row.)
local function shoot(me: Object, s: State, look: MegaManShotLook)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        look.buster.raise_arm(me)
        me.step_init = 4
        me:set_status("using_action", true)
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(look.shot_sound)
        local variant = s.projectile or look.plain
        me.attack_count = look.projectiles.fire(me, { variant = variant, damage = me.attack_damage, z = SHOT_Z })
        attachment.attach(look.attachment, me, look.buster.flash, "related")
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EBE20`: wait; then the shot starts at once. (Its entry also
-- calls `nullsub_12` with the panel in front.) BN5's (0x080EC9F0) waits in
-- the own-gauges mode alone (`look.waits`), the shot else starting on
-- its first tick.
local function wait(me: Object, s: State, look: MegaManShotLook)
    if me.step_init == 0 then
        if look.waits ~= nil and not look.waits(me) then
            me.step = SHOOT
            me.step_init = 0
            shoot(me, s, look)
            return
        end
        s.timer = WAIT_TICKS
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left <= 0 then
        me.step = SHOOT
        me.step_init = 0
        shoot(me, s, look)
    end
end

-- `sub_80EBEB2`: the recovery; a move may cut it short, even on the tick
-- it ends (there is no return after the exit).
local function recover(me: Object, s: State, look: MegaManShotLook)
    if me.step_init == 0 then
        s.timer = me.attack_count
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
    look.buster.move_cancel(me)
end

-- `sub_80EBE00`.
local function update(me: Object, s: State, look: MegaManShotLook)
    local step = me.step
    if step == WAIT then
        wait(me, s, look)
    elseif step == SHOOT then
        shoot(me, s, look)
    elseif step == RECOVER then
        recover(me, s, look)
    else
        error(string.format("charged shot phase %#x reads past its phase table", step))
    end
end

-- A game's charged shot, keyed `look.id`.
function charged_shot.action(look: MegaManShotLook): Action
    return define.action {
        id = look.id,
        state = { timer = "u16", projectile = "record:projectile-variant" },
        update = function(me: Object, s: State)
            update(me, s, look)
        end,
    }
end

return charged_shot
