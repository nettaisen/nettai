--!strict
-- GyroMan in the air: his own tick (0x080F09EC, the tick table's entry for him), his own idle (0x080F0978) and his own
-- action 9 (0x080F0C0C). He has FloatShoes and AirShoes, so he can stand where there is no panel; his tick watches the
-- panel under him. Over one that isn't solid he goes into the air (the navi state `hovering`) and asks to take off;
-- in the air, standing (animation 0), he flies (animation 20); in the air over a solid panel he asks to land, each
-- tick. His idle answers before the common idle: his action 9, which takes off (variant 0: animation 18 and a sound,
-- 18 ticks, then flying) or lands (variant 1: animation 3 and the sound, 3 ticks, then standing, out of the air, the
-- request cleared). Both ends leave his animation as they set it. While he is in the air a step sets no animation
-- (the move's own test of the state), and his buster is another (./buster).
--
-- The tick does nothing during his Airforce (his action 7: `during`, which ./init gives it). Its last part tests
-- the soul (NaviStats +0x2C) for GyroSoul's propeller, as MegaMan's tick does: never his.

local hover = {}

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
}

local SOLID = 0x10
local ANIM_FLY, ANIM_TAKE_OFF, ANIM_LAND = 20, 18, 3
local TAKE_OFF_TICKS, LAND_TICKS = 18, 3
-- (0x14C: 0x080F0C50 loads it as 77 + 255.)
local SOUND = asset.sound("gyro-change")

-- The steps, on the attack's step.
local CHOOSE, TAKE_OFF, LAND = 0x0, 0x4, 0x8

local function count(s: State): boolean
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    return t <= 0
end

hover.action = new.action {
    id = "gyroman/hover/action",
    state = { timer = "u16" },
    -- 0x080F0C0C
    update = function(me: Object, s: State)
        local step = me.step
        if step == CHOOSE then
            -- 0x080F0C2C: by the attack's variant.
            me.step, me.step_init = if me.attack_variant == 0 then TAKE_OFF else LAND, 0
        elseif step == TAKE_OFF then
            -- 0x080F0C40
            if me.step_init == 0 then
                me.step_init = 4
                me.anim = ANIM_TAKE_OFF
                battle.play_sound(SOUND)
                s.timer = TAKE_OFF_TICKS
            elseif count(s) then
                me.anim = ANIM_FLY
                me:end_attack()
            end
        elseif step == LAND then
            -- 0x080F0C70
            if me.step_init == 0 then
                me.step_init = 4
                me.anim = ANIM_LAND
                battle.play_sound(SOUND)
                s.timer = LAND_TICKS
            elseif count(s) then
                me.anim = 0
                me:set_navi_state("hovering", false)
                me:set_request("land", false)
                me:end_attack()
            end
        else
            error(string.format("GyroMan's take-off and landing's step %#x reads past its table (0x080F0C20)", step))
        end
    end,
}

-- 0x080F09EC: his tick, which does nothing while he runs `during` (his Airforce).
function hover.tick(during: Action): (navi: Object) -> ()
    return function(navi: Object)
        if navi:navi_action() == during then
            return
        end
        local solid = bit32.band(field.flags(navi.panel_x, navi.panel_y), SOLID) ~= 0
        local hovering = navi:navi_state("hovering")
        if not solid then
            if not hovering then
                navi:set_navi_state("hovering", true)
                navi:set_request("take_off", true)
            elseif navi.anim == 0 then
                navi.anim = ANIM_FLY
            end
        elseif hovering then
            navi:set_request("land", true)
        end
    end
end

-- 0x080F0978: his idle: a take-off or a landing asked for starts his action 9 in its variant.
function hover.idle(navi: Object)
    if navi:request("take_off") then
        navi:set_request("take_off", false)
        navi.attack_variant = 0
        navi:set_attack(hover.action, 0)
    elseif navi:request("land") then
        navi:set_request("land", false)
        navi.attack_variant = 1
        navi:set_attack(hover.action, 0)
    end
end

return hover
