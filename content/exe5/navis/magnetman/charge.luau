--!strict
-- MagnetMan's charged attack (`magnetman/charge`): the original's weapon routine 0x26 (0x0800F9BA) sets the damage his
-- level gives (the damage rows' row 8, lib/navi_level), his own counter byte, Elec and no target, and starts his own
-- action 7 (0x080F1158): he aims (animation 12, a counter window of 10 ticks) for 5 ticks, then a magnet missile
-- (./missile, its variant 4: 20 HP) appears 20 pixels ahead of him; 30 ticks, then back to idle.

local missile = require("./missile")
local navi_level = require("../../lib/navi_level")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
}

local HIT_PARAM = 0x8A
-- The damage rows' row 8, by his level.
local DAMAGE: { number } = { 40, 40, 40, 50, 50, 60, 60 }
local ELEC = 3
local ANIM = 12
local COUNTER_TICKS, AIM_TICKS, AFTER_TICKS = 10, 5, 30
-- The missile's variant, and how far ahead of him it appears (16.16).
local VARIANT, AHEAD = 4, 0x14_0000

-- The steps, on the attack's step.
local AIM, FIRE, AFTER = 0x0, 0x4, 0x8

charge.action = new.action {
    id = "magnetman/charge/action",
    state = { timer = "u16" },
    -- 0x080F1158
    update = function(me: Object, s: State)
        local step = me.step
        if step == AIM then
            -- 0x080F1178
            if me.step_init == 0 then
                me.step_init = 4
                me.anim = ANIM
                me:open_counter_window(COUNTER_TICKS)
                s.timer = AIM_TICKS
                return
            end
            local t = s.timer - 1
            s.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                me.step, me.step_init = FIRE, 0
            end
        elseif step == FIRE then
            -- 0x080F11A2: the missile follows what the attack points at (nothing, from his weapon).
            local p = me.pos
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
            missile.spawn(me, int.i32(p.x + me.facing * AHEAD), p.y, me.attack_element, VARIANT, damage, me.face_target)
            me.step, me.step_init = AFTER, 0
        elseif step == AFTER then
            -- 0x080F11CA
            if me.step_init == 0 then
                me.step_init = 4
                s.timer = AFTER_TICKS
                return
            end
            local t = s.timer - 1
            s.timer = bit32.band(t, 0xFFFF)
            if t <= 0 then
                me.anim = 0
                me:exit_attack()
            end
        else
            error(string.format("MagnetMan's charged attack's step %#x reads past its table (0x080F116C)", step))
        end
    end,
}

-- 0x0800F9BA
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = ELEC
    return charge.action
end

charge.weapon = new.weapon {
    id = "magnetman/charge",
    -- The charge table's row 0x26 (0x0801CBE8), by Charge 0 to 5: past 4 it reads on into row 0x27's first entry.
    charge_ticks = { 70, 65, 60, 55, 50, 120 },
    setup = setup,
}

return charge
