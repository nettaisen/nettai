--!strict
-- FireBrn1/2/3 and WideBrn1/2/3: action 0x27 (`sub_80ECD28`). The navi
-- holds out the burner (attachment 0x16, or 0x1E for a variant other than
-- 0) and lights a flame (attack object #0x04, objects/flame) on the panel
-- ahead with the chip's parameters: it burns Param1 ticks and spreads
-- forward Param2 panels (three rows for WideBrn), cracking the panels it
-- leaves. The navi keeps the pose for Param1 ticks, the burner roaring
-- every 16, then recovers for 6.

local attachment = require("../../objects/attachment/attachment")
local flame = require("../../objects/flame/flame")

export type State = {
    -- Ticks of the pose so far, then the recovery's (AIAttackVars+0x10).
    timer: number,
    -- The burner's roar, every 16 ticks (AIAttackVars+0x12). The game
    -- doesn't set it first: it continues from what the last action left
    -- there, so the first roar's tick depends on that action.
    roar: number,
}

local chip = {
    state = { timer = "u16", roar = "u16" },
}

-- The phases, on the attack's step.
local BURN, RECOVER = 0x0, 0x4

-- The burner (data.objects.attachments), for variant 0 and the others.
local BURNER, OTHER_BURNER = 0x16, 0x1E
local SOUND_ROAR = 0x12B

-- `sub_80ECD44`: light the flame, then hold the pose for Param1 ticks.
local function burn(me: Object, s: State)
    if me.step_init == 0 then
        -- The Atk+ bonus goes into the attack's damage word.
        local word = me.attack_damage + me.hit_param * 0x10000 + me.extra
        me.attack_damage = bit32.band(word, 0xFFFF)
        me.hit_param = bit32.rshift(word, 16)
        local params = { me:attack_param(1), me:attack_param(2), me:attack_param(3), me:attack_param(4) }
        flame.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, params, word)
        me.anim = 0x0A
        me:open_counter_window()
        attachment.spawn(me, { kind = if me.variant == 0 then BURNER else OTHER_BURNER, anim = 1 }, "overlay")
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    local roar = bit32.band(s.roar + 1, 0xF)
    s.roar = roar
    if roar == 1 then
        battle.play_sound(SOUND_ROAR)
    end
    local t = s.timer + 1
    s.timer = t
    if t >= me:attack_param(1) then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ECDD6`: 6 ticks, then the burner goes and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end
end

-- `sub_80ECD28`.
function chip.update(me: Object, s: State)
    local step = me.step
    if step == BURN then
        burn(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("FireBrn phase %#x reads past its phase table", step))
    end
end

return chip
