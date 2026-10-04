--!strict
-- MeddySoul's capsules on BN5's custom screen (docs/design/bn5-map.md §15.8; the screen's slots 8 and 9 in soul 6,
-- kinds 6 and 7: 0x08023D20). Each screen offers two of the five (BN5's chips 0x17C to 0x180), drawn by the console's
-- RNG1. A capsule is mixed into the last pick, a chip that deals damage and has none yet (0x08024C74): the screen's
-- state 0x3C (0x0802373A: the capsule's icon flies to the column, the soul's choice's flashes), then the chip's pick
-- carries the capsule's effect into the hand (the hand's flag byte, +68: 0x08023824) and the capsule is used. B on
-- the chip frees it (0x08024D78, the screen's). Their state is the souls system's (./system).

local chips = require("../../chips/yelcapsl/chips")

local capsules = {}

-- The five capsules (chips/yelcapsl).
local yelcapsl, blkcapsl, whicapsl, prpcapsl, pnkcapsl =
    chips.yelcapsl, chips.blkcapsl, chips.whicapsl, chips.prpcapsl, chips.pnkcapsl

-- The souls system's state fields the capsules keep.
capsules.state = {
    -- The screen's two capsules (its +0x26 and +0x28).
    capsule_1 = "chip",
    capsule_2 = "chip",
    -- The capsule being mixed (1 or 2), and its sequence's step and count (0x08023754's offsets 0 to 0x18; the
    -- screen's +0x40).
    mix_capsule = "u8",
    mix_step = "u8",
    mix_count = "u8",
}

type State = {
    capsule_1: Chip?,
    capsule_2: Chip?,
    mix_capsule: number,
    mix_step: number,
    mix_count: number,
}

-- MeddySoul, by number (0x08023D20).
local MEDDY = 6
-- The slots the capsules take: the hand's last two.
local SLOTS = { 8, 9 }
local BUTTONS = { "capsule_1", "capsule_2" }

-- What each capsule mixes into a chip (0x08023824): the hand's modifier bits, which the chip's use turns into its
-- damage word's confusion (0x2000), blindness (0x1000), paralysis (0x4000) or HP bug (0x0800), 0x08010368, or into
-- its user's heal, a tenth of its HP (0x0800FFF6); the chip-use rules' `mixed_modifiers`.
-- (By the capsule's id: a chip the state holds is its definition, not this module's table.)
local MIXES: { [string]: number } = {
    yelcapsl = 0x04,
    blkcapsl = 0x08,
    whicapsl = 0x02,
    prpcapsl = 0x20,
    pnkcapsl = 0x10,
}
-- The capsules a screen may offer (0x08025E60), and under a quarter of the HP (0x08025E80).
local OFFERED = {
    whicapsl, yelcapsl, blkcapsl, prpcapsl, pnkcapsl, blkcapsl, blkcapsl, whicapsl,
    blkcapsl, pnkcapsl, yelcapsl, whicapsl, yelcapsl, whicapsl, yelcapsl, whicapsl,
}
local OFFERED_LOW = {
    pnkcapsl, yelcapsl, blkcapsl, whicapsl, prpcapsl, whicapsl, yelcapsl, blkcapsl,
    pnkcapsl, whicapsl, yelcapsl, pnkcapsl, pnkcapsl, pnkcapsl, blkcapsl, pnkcapsl,
}

-- MegaMan is in MeddySoul (the screen's +0x10: the side's navi is MegaMan; NaviStats +0x2C).
local function in_soul(side: number): boolean
    local stats = battle.navi(side)
    local soul = stats.form.soul
    return stats.navi.id == "megaman" and soul ~= nil and soul.number == MEDDY
end

-- 0x08023D30, as the screen lays its slots out (after its deal, 0x08025114: the systems' deal hooks in turn, which
-- run before the screen's `open`): the screen's two capsules (none out of MeddySoul: the screen's record is zeroed
-- as it opens, 0x08022CA2), one draw of the console's RNG1, its bits 1 to 4 the first's place in the table and bits
-- 17 to 20 the second's; the table by whether MegaMan's HP is under a quarter (his maximum shifted right twice).
function capsules.deal(side: number)
    local s = system.state() :: State
    s.capsule_1, s.capsule_2 = nil, nil
    if not in_soul(side) then
        return
    end
    local navi = battle.player(side) :: Object
    local list = if navi.hp < bit32.rshift(navi.max_hp, 2) then OFFERED_LOW else OFFERED
    local r = battle.console_rng_positive(side)
    s.capsule_1 = list[bit32.rshift(bit32.band(r, 30), 1) + 1]
    s.capsule_2 = list[bit32.rshift(bit32.band(bit32.rshift(r, 16), 30), 1) + 1]
end

-- 0x08024C74: a capsule goes into the last pick: a chip that deals damage, with no capsule yet.
local function state(side: number): "selectable" | "unavailable"
    local last = custom.last_pick(side)
    if last == nil or last.attached or table.find(last.chip.flags or {}, "has_damage") == nil then
        return "unavailable"
    end
    return "selectable"
end

local function button(index: number): ButtonSpec
    return {
        slot = SLOTS[index],
        shown = function(side: number): boolean
            return in_soul(side)
        end,
        -- The capsule in the slot: its chip, which the chip window shows and R describes.
        chip = function(_side: number): Chip?
            local s = system.state() :: State
            return if index == 1 then s.capsule_1 else s.capsule_2
        end,
        state = state,
        -- 0x08024A02: the capsule's sequence.
        pressed = function(side: number)
            if custom.cursor_state(side) ~= "selectable" then
                custom.refuse(side)
                return
            end
            custom.play(side, "pick")
            local s = system.state() :: State
            s.mix_capsule, s.mix_step, s.mix_count = index, 0, 0
            custom.open_window(side, "capsule")
        end,
    }
end

capsules.buttons = { button(1), button(2) }

-- The sequence's fades' speed (`SetScreenFade(m, 32)`).
local FADE_SPEED = 32

-- The screen's state 0x3C (0x0802373A): the capsule's icon flies to the column (the soul's choice's steps: 16 ticks,
-- 8 sliding up, the flashes, the white); white, the last pick takes the capsule's effect and the capsule is used
-- (0x080237B4), and the screen clears; then the picking goes on. Each step after the flash waits for the fade before
-- it.
capsules.window = {
    update = function(side: number): boolean
        local s = system.state() :: State
        local fading = custom.fading(side)
        local step, count = s.mix_step, s.mix_count
        if step == 0 then
            -- 0x08023770
            step, count = 4, 0
        elseif step == 4 then
            -- 0x08023360
            count += 1
            if count >= 16 then
                step, count = 8, 0
            end
        elseif step == 8 then
            -- 0x0802337A
            count += 1
            if count >= 8 then
                custom.fade(side, "soul_flash", FADE_SPEED)
                custom.play(side, "program_advance_part")
                step, count = 12, 0
            end
        elseif step == 12 then
            -- 0x080233A8
            if not fading then
                custom.fade(side, "soul_flash_back", FADE_SPEED)
                step, count = 16, 0
            end
        elseif step == 16 then
            -- 0x080233C4
            if not fading then
                custom.fade(side, "end_to_white", FADE_SPEED)
                step, count = 20, 0
            end
        elseif step == 20 then
            -- 0x080237B4
            if not fading then
                local chip = (if s.mix_capsule == 1 then s.capsule_1 else s.capsule_2) :: Chip
                local mix = MIXES[chip.id]
                if mix == nil then
                    error(string.format("%s is no capsule (0x08023824 has five entries)", chip.id))
                end
                custom.attach_to_last_pick(side, BUTTONS[s.mix_capsule], mix)
                custom.update_availability(side)
                custom.fade(side, "intro_from_white", FADE_SPEED)
                custom.play(side, "program_advance")
                step, count = 24, 0
            end
        elseif step == 24 then
            -- 0x0802382C
            if not fading then
                custom.draw_emblem(side, 0)
                return false
            end
        else
            error(string.format("the capsule's sequence's step %#x reads past its table (0x08023754)", step))
        end
        s.mix_step, s.mix_count = step, count
        -- (The count is the screen's frame counter, +0x40.)
        custom.set_frame(side, count)
        custom.draw_emblem(side, 0)
        return true
    end,
}

return capsules
