--!strict
-- ColonelSoul's Arm Change on EXE5's custom screen (docs/design/exe5-map.md §15.8; the screen's slots 8 and 9 in
-- ColonelSoul, kinds 8 and 9: 0x08023D74): what the soul adds to the screen (its form's `custom`, which the souls
-- part gathers: rules/souls/custom). The last pick, a standard Null chip that deals damage and neither dims nor is
-- dark (0x08024C00), leaves the picks for the button (the screen's state 0x38, 0x08023694) and is the turn's arm chip
-- (the transform record's +6, 0x080123FC): at the turn's start ColonelSoul's charged shot fires it (0x080124AE:
-- AIData +0x32, the weapon routine 0x13: ./arm), for that turn. The chip leaves the folder. B, with the picks as
-- they were, puts it back (0x08024CFC, the screen's).

local arm = require("./arm")

local arm_change = {}

-- What Arm Change keeps of a screen (the rules' state's `screen`, rules/souls).
type Screen = {
    -- Its sequence's step (0x080236B8's offsets 0 and 4) and timer (the screen's +0x40).
    arm_step: number,
    arm_timer: number,
}
-- The rules' state: the screen's record, and the rules' own field it fills (rules/souls): the turn's arm chip (the
-- transform record's +6; none: its 0xFFFF), which each screen's open clears and the soul change copies into the
-- navi's weapon chip.
type State = {
    screen: Screen,
    arm_chip: Chip?,
}

-- The chip's icon blinks for this long.
local BLINK = 30

local function has(chip: Chip, flag: string): boolean
    return table.find(chip.flags or {}, flag) ~= nil
end

-- The button: two cells wide, leading on to the special slot and back to slot 7.
local button: SoulButton = {
    slot = 8,
    cells = 2,
    right = 11,
    left = 7,
    -- 0x08024C00: on offer for a last pick that is a standard chip of no family, dealing damage, not dimming, not
    -- dark.
    state = function(side: number, _s: any): "selectable" | "unavailable"
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
    pressed = function(side: number, s: any)
        if custom.cursor_state(side) ~= "selectable" then
            custom.refuse(side)
            return
        end
        custom.play(side, "pick")
        local st = (s :: State).screen
        st.arm_step, st.arm_timer = 0, 0
        custom.open_window(side, "arm_change")
    end,
}

-- The screen's state 0x38 (0x08023694): the last pick leaves the picks for the button, which is picked (0x080236C0);
-- then its icon blinks in the column cell it left for 30 ticks (0x080236EC: shown on the timer's odd ticks, and from
-- 20 down where its bit 1 is set), gone at the end; then the picking goes on.
local window: SoulWindow = {
    update = function(side: number, s: any): boolean
        local st = (s :: State).screen
        if st.arm_step == 0 then
            custom.hold_last_pick(side, "arm_change")
            st.arm_step, st.arm_timer = 4, BLINK
        else
            local t = st.arm_timer
            local shown = bit32.btest(t, if t > 20 then 1 else 2)
            custom.set_held_icon(side, "arm_change", shown)
            -- From 20 down, the ticks the column hides it draw it over the button (0x08023700).
            if t <= 20 and not shown then
                custom.draw_held(side)
            end
            t -= 1
            st.arm_timer = t
            if t == 0 then
                custom.update_availability(side)
                custom.set_frame(side, 0)
                custom.draw_emblem(side, 0)
                custom.draw_regular(side)
                return false
            end
        end
        -- (The timer is the screen's frame counter, +0x40. 0x080236A8: the emblem and the Regular chip's frame,
        -- after the step.)
        custom.set_frame(side, st.arm_timer)
        custom.draw_emblem(side, 0)
        custom.draw_regular(side)
        return true
    end,
}

local custom_screen: SoulCustom = {
    state = { arm_step = "u8", arm_timer = "u8" },
    buttons = { arm_change = button },
    windows = { arm_change = window },
    -- OK (0x08024EC0): the chip the button holds is the turn's arm chip (0x08025042).
    confirmed = function(side: number, s: any)
        (s :: State).arm_chip = custom.held_pick(side, "arm_change")
    end,
    -- The turn's start, before either side's change of form (0x08011DDC, 0x080124AE): the turn's arm chip (AIData
    -- +0x32) and the charged shot's weapon: the chip's (routine 0x13), or without one the soul's own (0x14).
    turn_opened = function(side: number, s: any)
        local navi = battle.player(side)
        if navi == nil then
            return
        end
        local chip = (s :: State).arm_chip
        navi.weapon_chip = chip
        local weapons = battle.navi(side).form.weapons
        navi.charge_shot_weapon = if chip ~= nil then arm.weapon elseif weapons ~= nil then weapons.charge_shot else nil
    end,
}

arm_change.custom = new.record("soul-custom", custom_screen)

return arm_change
