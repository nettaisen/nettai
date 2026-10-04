--!strict
-- The dragons' action (the original's 0x51, `sub_80EF4B4`; chips.md §3.8):
-- MegaMan raises his arm; on the third tick a dragon (./head, the chip's
-- variant) rises in the first column ahead holding an enemy's body, or the
-- column right ahead if none does, and dives down it. 16 ticks, then 6 of
-- recovery. Each chip composes its own (`action { dragon = ... }`).

local head = require("./head")
local panels = require("@exelib/panels")

local action = {}

export type State = {
    -- Ticks left in the current phase (+0x10).
    timer: number,
}

-- One state for every dragon: a use continues the last one's attack
-- variables, as the original's one action does.
action.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local SUMMON, RECOVER = 0x0, 0x4

local ANIM_RAISE = 0x0C
local SUMMON_TICKS, SUMMON_AT, RECOVER_TICKS = 0xF, 0xD, 5
-- The dragon rises from the row above the field.
local ROW = 0

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- A chip's dragon.
function action.action(spec: DragonSpec): Action
    -- `sub_80EF4D0`: raise the arm; two ticks in, the dragon.
    local function summon(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_RAISE)
            me:open_counter_window()
            me:set_status("using_action", true)
            s.timer = SUMMON_TICKS
            me.step_init = 4
        end
        if s.timer == SUMMON_AT then
            -- `sub_80ED040` (MachGun's too): without an enemy, the column
            -- right ahead.
            local x, found = panels.enemy_column(me)
            if not found then
                x = me.panel_x + me.facing
            end
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            head.spawn(me, x, ROW, me.attack_element, spec.dragon, damage)
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            s.timer = RECOVER_TICKS
            set_phase(me, RECOVER)
        end
    end

    -- `sub_80ECA0C`: wait, then back to idle.
    local function recover(me: Object, s: State)
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80EF4B4`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == SUMMON then
            summon(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("dragon chip phase %#x reads past its phase table", step))
        end
    end

    return define.action { id = spec.id, state = action.STATE, args = spec, update = update }
end

return action
