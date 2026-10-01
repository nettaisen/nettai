--!strict
-- DolThdr's action (the original's 0x3E, `sub_80EE0BC`): the navi holds
-- out its doll (or the buster arm, animation 0x19, when its chip says so:
-- the original's third parameter), and 10 ticks later the doll (attack
-- object #0x82, ./doll) strikes thunder along the row ahead, held for the
-- thunder's ticks (the original's second parameter); then the navi puts
-- it away over 7 ticks.

local attachment = require("../../objects/attachment/attachment")
local buster = require("../../lib/buster")
local doll = require("./doll")

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10), and the
    -- thunder's hold (AIAttackVars+0x12).
    timer: number,
    hold: number,
}

-- One state for every DolThdr: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16", hold = "u16" }

-- The phases, on the attack's step.
local RAISE, STRIKE, LOWER = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0A
local SOUND = asset.sound("beast-over")
-- What the navi holds: the doll (the original's attachment row 0x29), or
-- the buster arm (`dword_80EE29C`: row 6, animation 0x19).
local DOLL = attachment.look { sprite = asset.sprite("thunder-doll-hand"), palette = 0, attach_point = 0x0E }
local ARM_ANIM = 0x19
local RAISE_TICKS, HOLD_TICKS, LOWER_TICKS = 0x0A, 0x0F, 6

-- The held attachment's animation steps. Without one (its pool was full)
-- there is nothing to step.
local function step_held(me: Object, by: number)
    local held = me.overlay
    if held then
        held.anim += by
    end
end

-- A chip's DolThdr.
function action.action(spec: DollThunderSpec): Action
    -- `sub_80EE0DC`: the doll comes out.
    local function raise(me: Object, s: State)
        me:set_animation(ANIM_RAISE)
        me:open_counter_window()
        if spec.holds_arm then
            attachment.attach(me, buster.arm, "overlay", { anim = ARM_ANIM })
        else
            attachment.attach(me, DOLL, "overlay")
        end
        me:set_status("using_action", true)
        s.timer = RAISE_TICKS
        me.step = STRIKE
        me.step_init = 0
    end

    -- `sub_80EE110`: 10 ticks later the doll strikes; hold it.
    local function strike(me: Object, s: State)
        if me.step_init == 0 then
            local t = s.timer - 1
            s.timer = t
            if t > 0 then
                return
            end
            step_held(me, 1)
            s.hold = HOLD_TICKS
            me.step_init = 4
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            -- (The game hands the doll the navi's attack variables as its
            -- slot; DolThdr's first parameter is 0, so it never registers.)
            doll.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, spec.ticks, damage, nil)
            battle.play_sound(SOUND)
        end
        local h = s.hold - 1
        s.hold = h
        if h > 0 then
            return
        end
        me.step = LOWER
        me.step_init = 0
    end

    -- `sub_80EE162`: put it away; 7 ticks later, back to idle.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            step_held(me, -1)
            s.timer = LOWER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t >= 0 then
            return
        end
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
    end

    -- `sub_80EE0BC`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == STRIKE then
            strike(me, s)
        elseif step == LOWER then
            lower(me, s)
        else
            error(string.format("DolThdr phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
