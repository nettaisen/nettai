--!strict
-- CrakShot, DublShot and TrplShot: action 0x22 (`sub_80EC960`). The navi
-- digs into the panels ahead and flings them: on the third tick one crack
-- shot (attack object #0x33, objects/crack-shot) per panel of the chip's
-- region, the one panel ahead (CrakShot), two in a column (DublShot) or
-- three in a row (TrplShot). A panel that can't be dug (not solid, or
-- something on it) only cracks. 21 ticks in all.

local crack_shot = require("../../objects/crack-shot/crack_shot")

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10).
    timer: number,
}

local crak_shot = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local DIG, RECOVER = 0x0, 0x4

-- The region each variant digs (`dword_80ECA08`), from the panel ahead.
local REGIONS = { [0] = 1, 2, 4, 0 }
-- The tick (timer value) the shots fly.
local THROW_AT = 0x0D
-- Each shot's crossing budget (its Param1): it flies across at most 5
-- panel centers.
local CENTERS = 5

-- `sub_80EC97C`: dig, fling the shots on the timer's 0xD, then recover.
local function dig(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        battle.play_sound(0xD8)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = 0x0F
        me.step_init = 4
    end
    if s.timer == THROW_AT then
        local region = REGIONS[me.variant]
        if region == nil then
            error(string.format("CrakShot variant %d reads past dword_80ECA08", me.variant))
        end
        local panels = data.regions[region]
        if panels == nil then
            error(string.format("CrakShot region %d is not in the content", region))
        end
        local dir = me.facing
        -- The damage word (the hit parameter in its high half), plus the
        -- Atk+ bonus.
        local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
        -- The region is taken from the panel ahead.
        local x, y = me.panel_x + dir, me.panel_y
        for _, p in panels do
            crack_shot.spawn(me, x + dir * p[1], y + p[2], me.attack_element, damage, CENTERS)
        end
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        s.timer = 5
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ECA0C`: 6 ticks, then back to idle.
local function recover(me: Object, s: State)
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me:exit_attack()
    end
end

-- `sub_80EC960`.
function crak_shot.update(me: Object, s: State)
    local step = me.step
    if step == DIG then
        dig(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("CrakShot phase %#x reads past its phase table", step))
    end
end

return crak_shot
