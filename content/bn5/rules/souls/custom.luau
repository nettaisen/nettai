--!strict
-- Soul Unison on BN5's custom screen (docs/design/rules-in-luau.md §4.4, docs/design/bn5-map.md §15.8): the soul
-- button in the special slot (0x08023C54, its offer 0x08024B28, A on it 0x08024972), the soul's choice as the window
-- `soul_unison` (the screen's state 9, 0x080232D0), the soul given at OK (0x08024FF6) and the round's souls used
-- (0x02034E10). Its state is the souls system's (./system).

local operation_battle = require("../../lib/operation_battle")

local custom_souls = {}

-- The souls system's state fields the screen keeps (./system declares them).
type State = {
    -- 0x02034E10: the souls given this round: bit n Soul Unison with soul n, bit 16 + n its Chaos Unison (the
    -- round's first screen forgets them).
    souls_used: number,
    -- The button's offer (slot 11's +5 and +6): the soul (none: no offer), its number, and whether a dark chip makes
    -- it Chaos Unison. Once the soul is given, the soul given.
    offer: FormDef?,
    offer_number: number,
    offer_chaos: boolean,
    -- The window's step (0x080232F0's offsets 0 to 0x18) and its count (+0x40).
    unite_step: number,
    unite_count: number,
}

-- What the player's save brings (./system declares it): the souls it has (their flags, 0x08024BF0's, by each soul's
-- number), Soul Unison (event flag 0: the button) and Chaos Unison (event flag 0x236).
type Setup = {
    souls: { FormDef? },
    soul_unison: boolean,
    chaos_unison: boolean,
}

-- The sequence's fades' speed (`SetScreenFade(m, 32)`).
local FADE_SPEED = 32
-- A Soul Unison's turns (+ the NaviCust's and the cards' bonus, at most 9; under 0, 1), a Chaos Unison's.
local TURNS, MAX_TURNS, CHAOS_TURNS = 3, 9, 1

-- The side's navi's souls (MegaMan's: the forms with a `soul`).
local function souls_of(side: number): { FormDef }
    local forms = battle.navi(side).navi.forms
    if forms == nil or forms.souls == nil then
        return {}
    end
    return forms.souls
end

-- The soul a chip of `family` gives up for (the first of the navi's souls for that family).
local function soul_for(side: number, family: string): FormDef?
    for _, form in souls_of(side) do
        local soul = form.soul
        if soul ~= nil and soul.family == family then
            return form
        end
    end
    return nil
end

-- The setup's souls' places (./system's `form[16]`).
local SOUL_PLACES = 16

-- The save has soul `number` (its flag: one of the setup's souls of that number).
local function owned(number: number): boolean
    local setup = system.setup() :: Setup
    for i = 1, SOUL_PLACES do
        local form = setup.souls[i]
        if form ~= nil and form.soul ~= nil and form.soul.number == number then
            return true
        end
    end
    return false
end

-- A soul's bit in the round's souls used.
local function used_bit(number: number, chaos: boolean): number
    return bit32.lshift(1, if chaos then 16 + number else number)
end

-- 0x08024B28: the soul the last pick offers: a chip (not the Regular chip, not a link navi's) of a family one of the
-- navi's souls is for, a soul the save has (a dark chip's Chaos Unison needs the save's Chaos Unison), not given yet
-- this round (Soul Unison and Chaos Unison apart). None: (nil, 0, false).
local function offer(side: number): (FormDef?, number, boolean)
    local last = custom.last_pick(side)
    if last == nil or last.regular or last.navi_chip then
        return nil, 0, false
    end
    local chip = last.chip
    local chaos = table.find(chip.flags or {}, "dark") ~= nil
    if chaos and not (system.setup() :: Setup).chaos_unison then
        return nil, 0, false
    end
    local family = chip.family
    local form = if family ~= nil then soul_for(side, family) else nil
    if form == nil or form.soul == nil then
        return nil, 0, false
    end
    local number = form.soul.number
    if not owned(number) then
        return nil, 0, false
    end
    if bit32.btest((system.state() :: State).souls_used, used_bit(number, chaos)) then
        return nil, 0, false
    end
    return form, number, chaos
end

-- The soul button (0x08023C54), in the special slot.
custom_souls.button = {
    slot = 11,
    -- MegaMan with souls, outside the operation battle, with the save's Soul Unison, unless his emotion is worried or
    -- dark (0x08012740: in a soul or angry he may; at mood 0 he is dark, below 65 worried, at 0xFF Full Synchro).
    shown = function(side: number): boolean
        local stats = battle.navi(side)
        local in_soul = stats.form.soul ~= nil
        local mood = stats.mood
        local hidden = not in_soul
            and custom.player(side).emotion ~= "angry"
            and (mood == 0 or (mood ~= 0xFF and mood < 65))
        return #souls_of(side) > 0 and (system.setup() :: Setup).soul_unison and not operation_battle()
            and not hidden
    end,
    -- 0x08024B28: unavailable until the last pick offers a soul, which the button keeps.
    state = function(side: number): "selectable" | "unavailable"
        local s = system.state() :: State
        local form, number, chaos = offer(side)
        if form == nil then
            s.offer_number, s.offer_chaos = 0, false
            return "unavailable"
        end
        s.offer, s.offer_number, s.offer_chaos = form, number, chaos
        return "selectable"
    end,
    -- 0x08024972: a soul on offer starts its sequence.
    pressed = function(side: number)
        if custom.cursor_state(side) ~= "selectable" then
            custom.refuse(side)
            return
        end
        custom.play(side, "pick")
        local s = system.state() :: State
        s.unite_step, s.unite_count = 0, 0
        custom.open_window(side, "soul_unison")
    end,
} :: ButtonSpec

-- The screen's state 9 (0x080232D0): the soul's icon flies to the column (16 ticks, then 8 sliding up), the screen
-- flashes (fades 0x34 and 0x30) and whitens (fade 4); white, the soul takes the place of the chip given up for it,
-- first in the selection, the button is picked, and the screen clears (fade 0); then the picking goes on. Each step
-- after the flash waits for the fade before it.
custom_souls.window = {
    update = function(side: number): boolean
        local s = system.state() :: State
        local fading = custom.fading(side)
        local step, count = s.unite_step, s.unite_count
        if step == 0 then
            -- 0x0802330C
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
            -- 0x080233E0
            if not fading then
                custom.trade_last_pick(side, "soul")
                custom.set_button_state(side, "soul", "selected")
                custom.update_availability(side)
                custom.fade(side, "intro_from_white", FADE_SPEED)
                custom.play(side, "program_advance")
                step, count = 24, 0
            end
        elseif step == 24 then
            -- 0x08023452
            if not fading then
                custom.draw_emblem(side, 0)
                return false
            end
        else
            error(string.format("the soul's choice's step %#x reads past its table (0x080232F0)", step))
        end
        s.unite_step, s.unite_count = step, count
        -- (The count is the screen's frame counter, +0x40: the cursor blinks on from where it leaves it.)
        custom.set_frame(side, count)
        custom.draw_emblem(side, 0)
        return true
    end,
}

-- The round's first screen forgets the round's souls.
function custom_souls.open(side: number)
    if battle.turn() == 1 then
        (system.state() :: State).souls_used = 0
    end
end

-- 0x08024FF6: OK with the soul picked: the turn's form is the soul, for 3 turns and the save's bonus (at most 9;
-- under 0, 1), a Chaos Unison for 1; the soul is given this round.
function custom_souls.confirmed(side: number)
    if not custom.button_picked(side, "soul") then
        return
    end
    local s = system.state() :: State
    local turns = TURNS + battle.navi(side).soul_turn_bonus
    if s.offer_chaos then
        turns = CHAOS_TURNS
    elseif turns > MAX_TURNS then
        turns = MAX_TURNS
    elseif turns < 0 then
        turns = 1
    end
    custom.set_form(side, s.offer, turns, s.offer_chaos)
    s.souls_used = bit32.bor(s.souls_used, used_bit(s.offer_number, s.offer_chaos))
end

return custom_souls
