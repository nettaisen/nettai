--!strict
-- DolThdr's action (the original's 0x3E, `sub_80EE0BC`): the navi holds
-- out what its user says (DolThdr's doll; ElecCross's charged shot's buster
-- arm, animation 0x19: the original's third parameter), and 10 ticks later
-- the doll (attack
-- object #0x82, ./doll) strikes thunder along the row ahead, held for the
-- thunder's ticks (the original's second parameter); then the navi puts
-- it away over 7 ticks.

local attachment = require("../../objects/attachment")
local doll = require("./doll")

local action = {}

export type State = {
    -- Ticks left in the current phase (AIAttackVars+0x10). (The thunder's
    -- hold is the navi's `attack_count`.)
    timer: number,
}

-- One state for every DolThdr: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local RAISE, STRIKE, LOWER = 0x0, 0x4, 0x8

local ANIM_RAISE = 0x0A
local SOUND = asset.sound("beast-over")
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
        attachment.attach(me, spec.held, "overlay", { anim = spec.held_anim })
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
            me.attack_count = HOLD_TICKS
            me.step_init = 4
            local damage = me.attack_damage + me.hit_param * 0x10000 + me.extra
            -- (The game hands the doll the navi's attack variables as its
            -- slot; DolThdr's first parameter is 0, so it never registers.)
            doll.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, spec.ticks, damage, nil)
            battle.play_sound(SOUND)
        end
        local h = me.attack_count - 1
        me.attack_count = h
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

    return new.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
