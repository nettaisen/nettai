--!strict
-- HeatDrgn/ElecDrgn/AquaDrgn/WoodDrgn: action 0x51 (`sub_80EF4B4`). See
-- chips.md §3.8.
--
-- MegaMan raises his arm; on the third tick a dragon (objects/dragon-head,
-- of the chip's subtype) rises in the first column ahead holding an
-- enemy's body, or the column right ahead if none does, and dives down
-- it. 16 ticks, then 6 of recovery.

local dragon_head = require("../../objects/dragon-head/dragon_head")

export type State = {
    -- Ticks left in the current phase (+0x10).
    timer: number,
}

local dragons = {
    state = { timer = "u16" },
}

-- The phases, on the attack's step.
local SUMMON, RECOVER = 0x0, 0x4

local SUMMON_TICKS, SUMMON_AT, RECOVER_TICKS = 0xF, 0xD, 5
-- An enemy's body on a panel (with the dimming bit), by the user's side
-- (`byte_80ED080`).
local ENEMY_BODY = { [0] = 0x0401_0000, 0x0801_0000 }

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- `sub_80ED040`: the first column ahead with an enemy's body in it, and
-- whether there was one (without one, the last column ahead).
local function target_column(me: Object): (number, boolean)
    local dir = me.facing
    local body = ENEMY_BODY[me.alliance]
    local x = me.panel_x
    while true do
        x += dir
        if x == 0 or x >= 7 then
            return if dir > 0 then 6 else 1, false
        end
        if x < 0 then
            error("sub_80ED040 searches off the field's left edge (it would run on for 2^31 columns)")
        end
        for y = 1, 3 do
            if field.check(x, y, body, 0) then
                return x, true
            end
        end
    end
end

-- `sub_80EF4D0`: raise the arm; two ticks in, the dragon.
local function summon(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0C)
        me:open_counter_window()
        me:set_status("using_action", true)
        s.timer = SUMMON_TICKS
        me.step_init = 4
    end
    if s.timer == SUMMON_AT then
        local x, found = target_column(me)
        if not found then
            x = me.panel_x + me.facing
        end
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        dragon_head.spawn(me, x, 0, me.attack_element, me.variant, damage)
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
function dragons.update(me: Object, s: State)
    local step = me.step
    if step == SUMMON then
        summon(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("dragon chip phase %#x reads past its phase table", step))
    end
end

return dragons
