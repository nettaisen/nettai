--!strict
-- CornSht1-3: the navi raises its corn gun and, on the tenth tick, fires a
-- corn (corn.luau) from its own panel; where the corn finds something it
-- bursts twice, grasses the panel, and spreads to the enemies in the
-- column beyond (the original's action 0x2A, `sub_80ED090`;
-- docs/engine/shot-chips.md §9). The three differ only in their damage.
--
-- These are their actions; the chips are ./chips.

local attachment = require("../../objects/attachment/attachment")
local corn = require("./corn")

local cornsht = {}

export type State = {
    -- The phase's count (AIAttackVars+0x10).
    timer: number,
}

cornsht.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4

local ANIM_HOLD = 0xA
-- The corn leaves when the count reaches this; the recovery's ticks.
local FIRE_TICKS, RECOVER_TICKS = 0xA, 0x18
local SOUND = asset.sound("bn6:corn-shot")

-- The corn gun (the original's attachment row 0x18).
local GUN = attachment.look { sprite = asset.sprite("bn6:corn-shooter"), palette = 0, attach_point = 25 }

local function action(id: string): Action
    -- `sub_80ED0AC`: the Atk+ bonus goes into the damage word; the gun;
    -- on the tenth tick the corn, from the navi's own panel, the gun's
    -- next animation, the sound.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            local word = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            me.attack_damage = bit32.band(word, 0xFFFF)
            me.hit_param = bit32.rshift(word, 16)
            me.anim = ANIM_HOLD
            me:open_counter_window()
            attachment.attach(me, GUN, "overlay")
            me:set_status("using_action", true)
            s.timer = 0
            me.step_init = 4
        end
        local t = s.timer + 1
        s.timer = t
        if t < FIRE_TICKS then
            return
        end
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000)
        -- Its generation is the chips' first parameter, 0: the probe.
        corn.spawn(me, me.panel_x, me.panel_y, 0, me.attack_element, damage, 0)
        local gun = me.overlay
        if gun then
            gun.anim += 1
        end
        battle.play_sound(SOUND)
        me.step, me.step_init = RECOVER, 0
    end

    -- `sub_80ED118`: 25 ticks, then the gun goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
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

    return define.action {
        id = id,
        state = cornsht.STATE,
        -- `sub_80ED090`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("CornSht phase %#x reads past off_80ED0A4", step))
            end
        end,
    }
end

-- The corn gun, which CornFsta's farmer (chips/cornfsta) holds too.
cornsht.gun = GUN

cornsht.actions = {
    cornsht1 = action("bn6:cornsht1/action"),
    cornsht2 = action("bn6:cornsht2/action"),
    cornsht3 = action("bn6:cornsht3/action"),
}

return cornsht
