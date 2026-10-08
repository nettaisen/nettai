--!strict
-- GyroMan's own chip, Airforce (EXE5 chip 0x192), which his custom screen offers until it is picked (the navi's
-- `own_chip`): his own action 7 (0x080F0A74). He goes from the field (animation 4 on a solid panel, off his panel's
-- collision, 4 ticks; then off the field with his panel reserved and the chips' icons hidden) and flies his row
-- from his side's back column as the GyroMan chips' navi does (chips/gyroman/navi, Airforce's own kind of it: a bomb
-- on every enemy panel he passes, no bombing run), with the attack's damage and bonus; when the flight is gone he is
-- back on his panel (animation 3 on a solid one; 4 ticks) and to idle with his animation left as set. His tick
-- (./hover) rests meanwhile. Its damage is by his level (the chips' damage formula 52: the damage rows' row 3).
--
-- Off the field his panel is column 0 and, for its row, a byte the reservation leaves in a register (0x0801865C):
-- the low byte of his panel's flags with the reservation's bit (the game's number of the panel's type, and its
-- solid, side and cracked bits), or 0 had another reserved the panel he stands on.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local flight = require("../../chips/gyroman/navi")
local regions = require("@exelib/regions")

-- His flight (the chips' GyroMan's object with its fourth parameter 1): from the take-off, no bombing run, standing
-- still while the battle is dimmed.
local FLIGHT = flight.make { id = "gyroman/airforce", appears = false, runs = false, while_dimmed = false }

export type State = {
    -- AIAttackVars +0x10: the step's ticks left.
    timer: number,
    -- AIAttackVars +0x30: the flight.
    flight: Object?,
}

-- The steps, on the attack's step.
local LEAVE, FLY, AWAY, RETURN = 0x0, 0x4, 0x8, 0xC
local SOLID = 0x10
local ANIM_LEAVE, ANIM_RETURN = 4, 3
local LEAVE_TICKS, RETURN_TICKS = 4, 4

local function solid(me: Object): boolean
    return bit32.band(field.flags(me.panel_x, me.panel_y), SOLID) ~= 0
end

local function count(s: State): boolean
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    return t <= 0
end

local action = new.action {
    state = { timer = "u16", flight = "object" },
    -- 0x080F0A74
    update = function(me: Object, s: State)
        local step = me.step
        if step == LEAVE then
            -- 0x080F0A98
            if me.step_init == 0 then
                me.step_init = 4
                if solid(me) then
                    me.anim = ANIM_LEAVE
                end
                me.collision:set_region(nil)
                s.timer = LEAVE_TICKS
            elseif count(s) then
                local x, y = me.panel_x, me.panel_y
                me.future_panel_x, me.future_panel_y = x, y
                local reserved = me:reserve_panel(x, y)
                me.panel_x, me.panel_y = 0, if reserved then bit32.band(field.flags(x, y), 0xFF) else 0
                me:set_coordinates_from_panel()
                battle.show_hud({ "chip_icons" }, false)
                me.step, me.step_init = FLY, 0
            end
        elseif step == FLY then
            -- 0x080F0AEA: from his side's back column, along the row he left.
            local x = if me.alliance == 0 then 1 else 6
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            s.flight = flight.fly(FLIGHT, me, x, me.future_panel_y, me.attack_element, damage)
            me.step, me.step_init = AWAY, 0
        elseif step == AWAY then
            -- 0x080F0B0C: until the flight is going (a flight that never came keeps him away).
            local f = s.flight
            if f and f.lifecycle == "destroy" then
                me.step, me.step_init = RETURN, 0
            end
        elseif step == RETURN then
            -- 0x080F0B20
            if me.step_init == 0 then
                me.step_init = 4
                me.collision:set_region(regions.single)
                local x, y = me.future_panel_x, me.future_panel_y
                me.panel_x, me.panel_y = x, y
                me:unreserve_panel(x, y)
                me:set_coordinates_from_panel()
                battle.show_hud({ "chip_icons" }, true)
                if solid(me) then
                    me.anim = ANIM_RETURN
                end
                s.timer = RETURN_TICKS
            elseif count(s) then
                me:end_attack()
            end
        else
            error(string.format("GyroMan's Airforce's step %#x reads past its table (0x080F0A88)", step))
        end
    end,
}

local chip: Chip = {
    codes = { "G" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 50, 50, 50, 50, 70, 70, 70 }),
    hit_param = 138,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
}

return { airforce = chip }
