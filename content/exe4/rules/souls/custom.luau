--!strict
-- Double Soul's part of EXE4's custom screen, as its rules answer the framework's hooks (docs/design/exe4-map.md §5,
-- as EXE5's rules/souls/custom): the hand's size, which a soul changes (NumberSoul's ten, the souls' own: §18 item 74);
-- the UNITE button under OK (the selection's state 0xC, 0x08020728; its look 0x0801FF14) and the soul it offers
-- (0x0801FF84, the screen's +0x0D); A on it (0x0801F8BC) and the soul's choice, the window `double_soul` (the
-- selection's state 0x10, 0x08020814: EXE5's soul choice's steps, its own code); the soul's chip put first in the hand
-- at OK, its soul the turn's form (0x0801F0A0) and the soul given this round (0x0801F1FE: 0x0203BF88, which the
-- battle's start zeroes, 0x0801594E).

local dark_chips = require("../dark_chips")
local turns = require("./turns")

local custom_souls = {}

-- The rules' state fields the screen keeps (rules/init.luau declares them).
type State = {
    -- 0x0203BF88: the souls given this round (a bit a soul in the original), each by its form.
    souls_used: { FormDef },
    -- The button's offer (the screen's +0x0D: the soul, none without one), the soul given (+0x0C's 2: the button
    -- picked), and the soul's choice's step (+0x03) and count (+0x44).
    offer: FormDef?,
    unite_step: number,
    unite_count: number,
}

-- What the player's save brings (rules/init.luau declares it): Double Soul (event flag 0x14) and the souls the save
-- has (its soul flags, 0x08020018's by soul).
type Setup = {
    double_soul: boolean,
    souls: { FormDef? },
}

-- The setup's souls' places (./init's `form[12]`).
local SOUL_PLACES = 12
-- The soul's choice's fades' speed (`SetScreenFade(m, 32)`).
local FADE_SPEED = 32
-- A on the button with a soul on offer: OK's sound (0x080207EC[0]).
local OK_SOUND = asset.sound("custom-ok")

-- 0x0801FFBE: the soul a chip of `family` is given up for: the side's navi's form (of any of its lists) that is the
-- soul of that family (0x08020008's table by soul). No two souls are of one family.
local function soul_for(side: number, family: string): FormDef?
    local forms: { [string]: any } = battle.navi(side).navi.forms or {}
    for _, set in forms do
        local list: { FormDef } = set.form_list or {}
        for _, form in list do
            local soul = form.soul
            if soul ~= nil and soul.family == family then
                return form
            end
        end
    end
    return nil
end

-- The save has `soul` (its flag, 0x08020018's: one of the setup's souls).
local function owned(soul: FormDef): boolean
    local setup = rules.setup() :: Setup
    for i = 1, SOUL_PLACES do
        if setup.souls[i] == soul then
            return true
        end
    end
    return false
end

-- The soul was given this round (0x0203BF88's bit).
local function used(soul: FormDef): boolean
    local given = (rules.state() :: State).souls_used
    for i = 1, #given do
        if given[i] == soul then
            return true
        end
    end
    return false
end

-- 0x0801FFA2: the soul the last pick offers: a chip but the Regular chip (an entry of slot 0 while the folder's Regular
-- chip waits, BattleState +0x17), of a family one of the navi's souls is for, a soul the save has and that wasn't given
-- this round. None: nil.
local function offer(side: number): FormDef?
    local last = custom.last_pick(side)
    if last == nil or last.regular then
        return nil
    end
    local family = last.chip.family
    local soul = if family ~= nil then soul_for(side, family) else nil
    if soul == nil or not owned(soul) or used(soul) then
        return nil
    end
    return soul
end

-- The UNITE button (0x0801FF14, the screen's +0x0C: 0 none, 1 on the screen, 2 a soul given), under OK in slot 11.
custom_souls.button = {
    slot = 11,
    view = "form_offer",
    -- 0x0801E0B4, as the screen opens: with Double Soul (event flag 0x14), unless the side's navi is worried or worn
    -- out (the screen's +0x10, 0x0800F49C's 1 and 5). Without it the button's place shows the window (its look 2),
    -- and OK's DOWN goes nowhere (0x0802069E).
    shown = function(side: number): boolean
        local emotion = custom.player(side).emotion
        return (rules.setup() :: Setup).double_soul and emotion ~= "worried" and emotion ~= "worn_out"
    end,
    -- 0x0801FF84 after each pick (and 0x0801FF14's look): lit while the last pick offers a soul, gray while it doesn't.
    -- The offer is asked only before a soul is given (+0x0C 1: once given it is the soul given) and while the navi's
    -- turns are counted (AIData +0x18 not 0xFF: a navi that started the round in a soul, which no netbattle's does);
    -- else it stays as it was.
    state = function(side: number): "selectable" | "unavailable"
        local s = rules.state() :: State
        if not custom.button_picked(side, "soul") and turns.counted() then
            s.offer = offer(side)
        end
        return if s.offer == nil then "unavailable" else "selectable"
    end,
    -- 0x0801F8BC: a soul on offer, not given yet, starts its choice (the screen's +0x0C 2; OK's sound, 0x080207EC[0]);
    -- with none A is refused (0x080207EC[1]).
    pressed = function(side: number)
        if custom.cursor_state(side) ~= "selectable" then
            custom.refuse(side)
            return
        end
        custom.play_sound(side, OK_SOUND)
        local s = rules.state() :: State
        s.unite_step, s.unite_count = 0, 0
        custom.open_window(side, "double_soul")
    end,
    -- 0x0801F0A0: picked, the soul's chip goes first in the hand (0x160 + its number: the soul's form's `chip`).
    hand_chip = function(side: number): Chip?
        local s = rules.state() :: State
        local soul = s.offer
        return if soul ~= nil and custom.button_picked(side, "soul") then soul.chip else nil
    end,
} :: ButtonSpec

-- The selection's state 0x10 (0x08020814): the soul's icon (its chip's, 0x160 + the soul) flies to the column (16
-- ticks, then 8 sliding up 2 pixels a tick, 0x08020884, 0x0802089A), the screen flashes (fades 0x34 and 0x30, with
-- the Program Advance's part sound, 0x79) and whitens (fade 4); white, the soul takes the place of the chip given up
-- for it, first in the selection, the button is picked, and the screen clears (fade 0, with the Program Advance's
-- sound, 0x97: 0x080208F4); then the picking goes on (state 0xC, 0x0802095E). Each step after the flash
-- waits for the fade before it. Drawn as EXE5's (the window view `offer_flight`).
custom_souls.window = {
    view = "offer_flight",
    update = function(side: number): boolean
        local s = rules.state() :: State
        local fading = custom.fading(side)
        local step, count = s.unite_step, s.unite_count
        if step == 0 then
            -- 0x08020844
            step, count = 4, 0
        elseif step == 4 then
            -- 0x08020884
            count += 1
            if count >= 16 then
                step, count = 8, 0
            end
        elseif step == 8 then
            -- 0x0802089A
            count += 1
            if count >= 8 then
                count = 0
                custom.fade(side, "flash", FADE_SPEED)
                custom.play(side, "program_advance_part")
                step = 12
            end
        elseif step == 12 then
            -- 0x080208C4
            if not fading then
                custom.fade(side, "flash_back", FADE_SPEED)
                step = 16
            end
        elseif step == 16 then
            -- 0x080208DC
            if not fading then
                custom.fade(side, "end_to_white", FADE_SPEED)
                step = 20
            end
        elseif step == 20 then
            -- 0x080208F4
            if not fading then
                custom.trade_last_pick(side, "soul")
                dark_chips.last_pick_first(side)
                custom.set_button_state(side, "soul", "selected")
                custom.update_availability(side)
                custom.fade(side, "intro_from_white", FADE_SPEED)
                custom.play(side, "program_advance")
                step = 24
            end
        elseif step == 24 then
            -- 0x0802095E
            if not fading then
                return false
            end
        else
            error(string.format("the soul's choice's step %#x reads past its table (0x08020828)", step))
        end
        s.unite_step, s.unite_count = step, count
        return true
    end,
} :: WindowSpec

-- The screen opens (0x0801DC48 zeroes its block): no offer.
function custom_souls.open(_side: number)
    (rules.state() :: State).offer = nil
end

-- 0x0801F1FE, at OK with the soul picked: the turn's form is the soul (its turns are its own, ./turns), and the soul
-- is given this round.
function custom_souls.confirmed(side: number)
    if not custom.button_picked(side, "soul") then
        return
    end
    local s = rules.state() :: State
    local soul = s.offer :: FormDef
    custom.set_form(side, "soul", soul)
    s.souls_used[#s.souls_used + 1] = soul
end

-- The hand the screen deals as it opens (0x0801DC8E, its +0x11): the navi's custom level (NaviStats +0x12, 0x0801DCAE)
-- as it is, where EXE6's caps it at 8 and takes its hand-shrink bug from it. (The soul 8's ten, 0x0801DC92, is the
-- souls' own, §18 item 74; a battle type of 64 to 67 deals five, 0x0801DC98, and no netbattle is of one.)
function custom_souls.hand_size(side: number): number
    return battle.navi(side).custom_level
end

custom_souls.buttons = { soul = custom_souls.button }
custom_souls.windows = { double_soul = custom_souls.window }

return custom_souls
