--!strict
-- GyroMan's charged attack (`gyroman/charge`): the original's weapon routine 0x28 (0x0800F9D6) sets the damage his
-- level gives (the damage rows' row 2, lib/navi_level), his own counter byte and no element, and starts his own
-- action 8 (0x080F0B6E): he raises his arm (animation 17) for 4 ticks, then a tornado blows along the three panels
-- ahead (GyroSoul's, navis/megaman/forms/gyrosoul/tornado, lying on the ground; the action plays its sound too);
-- 30 ticks, then back to idle. No counter window opens. In the operation battle the panel ahead is marked as he
-- raises his arm (lib/navi_chips/marker).

local marker = require("../../lib/navi_chips/marker")
local navi_level = require("../../lib/navi_level")
local tornado = require("../megaman/forms/gyrosoul/tornado")

local charge = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
}

local HIT_PARAM = 0x8A
-- The damage rows' row 2, by his level: a hit's damage.
local DAMAGE: { number } = { 20, 20, 20, 30, 30, 30, 30 }
local ANIM = 17
local RAISE_TICKS, BLOW_TICKS = 4, 30
-- The operation battle's mark: 30 ticks, style 6.
local MARK_TICKS, MARK_STYLE = 30, 6
-- (0x103: 0x080F0BDA loads it as 4 + 255.)
local SOUND = asset.sound("gyrosoul-tornado")

-- The steps, on the attack's step.
local RAISE, BLOW = 0x0, 0x4

local function count(s: State): boolean
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    return t <= 0
end

charge.action = define.action {
    id = "gyroman/charge/action",
    state = { timer = "u16" },
    -- 0x080F0B6E
    update = function(me: Object, s: State)
        local step = me.step
        local ahead = me.panel_x + me.facing
        if step == RAISE then
            -- 0x080F0B88
            if me.step_init == 0 then
                me.step_init = 4
                me.anim = ANIM
                s.timer = RAISE_TICKS
                marker.spawn(me, ahead, me.panel_y, MARK_TICKS, MARK_STYLE, true)
            elseif count(s) then
                tornado.spawn(me, ahead, me.panel_y, int.u32(me.attack_damage + me.hit_param * 0x10000), true)
                battle.play_sound(SOUND)
                me.step, me.step_init = BLOW, 0
            end
        elseif step == BLOW then
            -- 0x080F0BE8
            if me.step_init == 0 then
                me.step_init = 4
                s.timer = BLOW_TICKS
            elseif count(s) then
                me.anim = 0
                me:exit_attack()
            end
        else
            error(string.format("GyroMan's charged attack's step %#x reads past its table (0x080F0B80)", step))
        end
    end,
}

-- 0x0800F9D6
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    return charge.action
end

charge.weapon = define.weapon {
    id = "gyroman/charge",
    -- The charge table's row 0x28 (0x0801CBFC), by Charge 0 to 5: past 4 it reads on into row 0x29's first entry.
    charge_ticks = { 80, 75, 70, 65, 60, 120 },
    setup = setup,
}

return charge
