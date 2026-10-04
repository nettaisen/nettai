--!strict
-- ColonelSoul's Arm Change on BN5's custom screen (docs/design/bn5-map.md §15.8; the screen's slots 8 and 9 in soul
-- 7, kinds 8 and 9: 0x08023D74). The last pick, a standard Null chip that deals damage and neither dims nor is dark
-- (0x08024C00), leaves the picks for the button (the screen's state 0x38, 0x08023694) and is the turn's arm chip (the
-- transform record's +6, 0x080123FC): at the turn's start ColonelSoul's charged shot fires it (0x080124AE: AIData
-- +0x32, the weapon routine 0x13), for that turn. The chip leaves the folder. B, with the picks as they were, puts
-- it back (0x08024CFC, the screen's). Its state is the souls system's (./init).

local arm = require("../../navis/megaman/forms/colonelsoul/arm")

local arm_change = {}

-- The souls system's state fields Arm Change keeps.
arm_change.state = {
    -- Its sequence's step (0x080236B8's offsets 0 and 4) and timer (the screen's +0x40).
    arm_step = "u8",
    arm_timer = "u8",
    -- The turn's arm chip (the transform record's +6; none: its 0xFFFF).
    arm_chip = "chip",
}

type State = {
    arm_step: number,
    arm_timer: number,
    arm_chip: Chip?,
}

-- ColonelSoul, by number (0x08023D74, 0x080124BC).
local COLONEL = 7
-- The chip's icon blinks for this long.
local BLINK = 30

-- MegaMan is in ColonelSoul (the screen's +0x10: the side's navi is MegaMan; NaviStats +0x2C).
local function in_soul(side: number): boolean
    local stats = battle.navi(side)
    local soul = stats.form.soul
    return stats.navi.id == "megaman" and soul ~= nil and soul.number == COLONEL
end

local function has(chip: Chip, flag: string): boolean
    return table.find(chip.flags or {}, flag) ~= nil
end

-- The button: two cells wide, leading on to the special slot and back to slot 7.
arm_change.button = {
    slot = 8,
    cells = 2,
    right = 11,
    left = 7,
    -- (By the soul the emotion routine leaves in r1, 0x08023C3C, which in battle mode 1 is something else,
    -- 0x080127C0: no button there.)
    shown = function(side: number): boolean
        return in_soul(side) and battle.mode() ~= 1
    end,
    -- 0x08024C00: on offer for a last pick that is a standard chip of no family, dealing damage, not dimming, not
    -- dark.
    state = function(side: number): "selectable" | "unavailable"
        local last = custom.last_pick(side)
        if last == nil then
            return "unavailable"
        end
        local chip = last.chip
        if chip.class == "standard" and chip.family == "null" and has(chip, "has_damage") and not has(chip, "dimming")
            and not has(chip, "dark")
        then
            return "selectable"
        end
        return "unavailable"
    end,
    -- 0x080249D6: the chip's sequence.
    pressed = function(side: number)
        if custom.cursor_state(side) ~= "selectable" then
            custom.refuse(side)
            return
        end
        custom.play(side, "pick")
        local s = system.state() :: State
        s.arm_step, s.arm_timer = 0, 0
        custom.open_window(side, "arm_change")
    end,
} :: ButtonSpec

-- The screen's state 0x38 (0x08023694): the last pick leaves the picks for the button, which is picked (0x080236C0);
-- then its icon blinks in the column cell it left for 30 ticks (0x080236EC: shown on the timer's odd ticks, and from
-- 20 down where its bit 1 is set), gone at the end; then the picking goes on.
arm_change.window = {
    update = function(side: number): boolean
        local s = system.state() :: State
        if s.arm_step == 0 then
            custom.hold_last_pick(side, "arm_change")
            s.arm_step, s.arm_timer = 4, BLINK
        else
            local t = s.arm_timer
            custom.set_held_icon(side, "arm_change", bit32.btest(t, if t > 20 then 1 else 2))
            t -= 1
            s.arm_timer = t
            if t == 0 then
                custom.update_availability(side)
                custom.set_frame(side, 0)
                custom.draw_emblem(side, 0)
                return false
            end
        end
        -- (The timer is the screen's frame counter, +0x40.)
        custom.set_frame(side, s.arm_timer)
        custom.draw_emblem(side, 0)
        return true
    end,
}

-- The screen opens: no arm chip for the turn (0x080123D4: the transform record's +6 0xFFFF).
function arm_change.open(_side: number)
    (system.state() :: State).arm_chip = nil
end

-- OK (0x08024EC0): the chip the button holds is the turn's arm chip (0x08025042).
function arm_change.confirmed(side: number)
    if in_soul(side) then
        (system.state() :: State).arm_chip = custom.held_pick(side, "arm_change")
    end
end

-- The turn's start, before either side's change of form (0x08011DDC, 0x080124AE): in ColonelSoul, the turn's arm
-- chip (AIData +0x32) and the charged shot's weapon: the chip's (routine 0x13), or without one the soul's own
-- (0x14).
function arm_change.turn_opened(side: number)
    local navi = battle.player(side)
    if navi == nil or not in_soul(side) then
        return
    end
    local chip = (system.state() :: State).arm_chip
    navi.weapon_chip = chip
    local weapons = battle.navi(side).form.weapons
    navi.charge_shot_weapon = if chip ~= nil then arm.weapon elseif weapons ~= nil then weapons.charge_shot else nil
end

return arm_change
