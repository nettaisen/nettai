--!strict
-- SearchSoul's SHUFFLE on EXE4's custom screen (its form's `custom`): a button two cells wide over slots 8 and 9 of
-- each screen opened in the soul (0x0801DD18: the slots' flags 9, its picture's look the screen's +0x13), three
-- shuffles a screen (+0x1A: the button's uses, which the chip window shows while the cursor is on it, 0x0801FEA2).
-- A on it (0x08020370) with a shuffle left takes one, makes the deal (0x0802019E) and starts the selection's state
-- 0x14 (0x08020998: the window `shuffle`); with none left it is refused (0x08020384).
--
-- The state runs 17 ticks. Every fourth, from its first, sounds (0x123) and draws the slots again: the first four
-- show the folder's chips not picked shuffled where they are (0x080200F4: 30 swaps), the fifth lands the deal
-- (0x08020232) and gives the button its look back, gray with no shuffle left; the button's cursor is drawn through
-- it (0x08020E08), and the chip window again at its end (0x0801FC00). The deal is of the hand's chips not
-- picked and the folder's after the hand: with any after the hand, the hand's are shuffled among themselves, then
-- all but the first few (the table 0x08020294, by how many of the hand it deals: those stay in the hand), and then,
-- or with the hand alone, the hand's once more. The picked chips and the Regular chip while it waits (BattleState
-- +0x17: 0x080202A4 marks its slot) stay where they are. Every shuffle is the console's (0x08000B4A: RNG1).
--
-- (R reads nothing on the button, 0x080203AC; LEFT leaves it as from slot 8 and RIGHT goes to OK, 0x0802044E and
-- 0x0802041C: the engine's two-cell button. EXE5's Shuffle, the framework's re-deal, is this routine's later form.)

local shuffle = {}

-- What the screen keeps (the rules' state): the deal (0x0203AC30).
type Screen = {
    shuffle_deal: { FolderChip },
}

-- The button's name: the pack's look of it.
local BUTTON = "redeal"
-- A screen's shuffles (0x0801DD2A).
local SHUFFLES = 3
-- The state's steps: one every 4 ticks, the fifth the deal's (0x080209A0, 0x080209BE).
local STEP, LAST = 4, 16
-- A shown shuffle's swaps (0x0802014A).
local SHOWN_SWAPS = 30
-- How many of the hand's chips stay in the hand, by how many of them are dealt again (0x08020294, and the three
-- bytes after it).
local KEPT: { [number]: number } = { [0] = 0, 0, 0, 1, 1, 2, 2, 3, 3, 0, 0, 0 }
-- A step's sound (0x080209DC).
local SOUND = asset.sound("hop")

-- 0x08000B4A: `swaps` swaps of two of `list`'s `n` entries from `first`, by two draws of the console's RNG1 each.
local function swapped(side: number, list: { FolderChip }, first: number, n: number, swaps: number)
    for _ = 1, swaps do
        local a = first + battle.console_rng_positive(side) % n
        local b = first + battle.console_rng_positive(side) % n
        list[a], list[b] = list[b], list[a]
    end
end

-- The folder's chips left (the screen's +0x15: it closed the folder up as it opened).
local function chips_left(side: number): number
    local n = 0
    for _, c in custom.folder(side) do
        if c then
            n += 1
        end
    end
    return n
end

-- The slots a shuffle leaves alone (their flags' 0x02 and 0x80): the picked chips' and the waiting Regular chip's
-- (0x080202A4, 0x08020102), in slot order, each by its place in the folder.
local function kept_places(side: number): { number }
    local places: { number } = {}
    for slot = 0, 9 do
        local here = custom.slot(side, slot)
        if here ~= nil and here.place ~= nil and (here.picked or here.regular) then
            table.insert(places, here.place)
        end
    end
    return places
end

-- The places the deal lands on, in its order: the hand's not kept (0x080202A4 counts them: the chips dealt less the
-- kept ones), then the folder's after the hand; and how many of them are the hand's.
local function dealt_places(side: number): ({ number }, number)
    local left, size = chips_left(side), custom.hand_size(side)
    local kept: { [number]: boolean } = {}
    for _, place in kept_places(side) do
        kept[place] = true
    end
    local places: { number } = {}
    for place = 1, math.min(size, left) do
        if not kept[place] then
            table.insert(places, place)
        end
    end
    local hand = #places
    if hand ~= 0 then
        for place = size + 1, left do
            table.insert(places, place)
        end
    end
    return places, hand
end

-- 0x0802019E: the deal.
local function make_deal(side: number, s: Screen)
    local places, hand = dealt_places(side)
    local folder = custom.folder(side)
    local deal: { FolderChip } = {}
    for i, place in places do
        deal[i] = folder[place] :: FolderChip
    end
    if hand ~= 0 then
        if #deal > hand then
            swapped(side, deal, 1, hand, hand)
            local stay = KEPT[hand]
            swapped(side, deal, stay + 1, #deal - stay, #deal - stay)
        end
        swapped(side, deal, 1, hand, hand)
    end
    s.shuffle_deal = deal
end

-- 0x080200F4: the chips not kept shown shuffled. The kept ones go to the folder's front, the rest are shuffled where
-- they are, and the kept ones go back, the last first.
local function show_shuffled(side: number)
    local kept = kept_places(side)
    for i, place in kept do
        custom.swap_folder(side, place, i)
    end
    local rest = chips_left(side) - #kept
    if rest ~= 0 then
        for _ = 1, SHOWN_SWAPS do
            local a = #kept + 1 + battle.console_rng_positive(side) % rest
            local b = #kept + 1 + battle.console_rng_positive(side) % rest
            custom.swap_folder(side, a, b)
        end
    end
    for i = #kept, 1, -1 do
        custom.swap_folder(side, kept[i], i)
    end
end

-- 0x08020232: the deal lands.
local function land(side: number, s: Screen)
    for i, place in dealt_places(side) do
        local c = s.shuffle_deal[i]
        custom.set_folder(side, place, c.chip, c.code)
    end
end

shuffle.custom = new.record("soul-custom", {
    state = {
        shuffle_deal = schema.list({ chip = "chip", code = "code" }, 30),
    },
    buttons = {
        [BUTTON] = {
            slot = 8,
            cells = 2,
            uses = SHUFFLES,
            -- 0x08020370.
            pressed = function(side: number, s: Screen)
                local left = custom.button_uses(side, BUTTON)
                if left == 0 then
                    custom.refuse(side)
                    return
                end
                custom.set_button_uses(side, BUTTON, left - 1)
                make_deal(side, s)
                custom.open_window(side, "shuffle")
                -- (The tick's draw is the new state's, 0x0801E41E: the button's cursor.)
                custom.draw_cursor(side)
            end,
        },
    },
    windows = {
        -- 0x08020998: the selection's state 0x14, its count (+3) the window's ticks before this one.
        shuffle = {
            update = function(side: number, s: Screen): boolean
                local count = custom.window_tick(side) - 1
                if count % STEP == 0 then
                    if count < LAST then
                        custom.set_button_state(side, BUTTON, "selected")
                        show_shuffled(side)
                    else
                        local left = custom.button_uses(side, BUTTON)
                        custom.set_button_state(side, BUTTON, if left == 0 then "unavailable" else "selectable")
                        land(side, s)
                    end
                    -- 0x0801F5EC, 0x0801F674, 0x0801F9E0, 0x0801FAE0: the slots as they are now.
                    custom.update_availability(side)
                    custom.play_sound(side, SOUND)
                end
                if count < LAST then
                    custom.draw_cursor(side)
                    return true
                end
                -- 0x080209EA: back to the chips (0x0801FC00: the chip window's).
                custom.show_chip_window(side)
                return false
            end,
        },
    },
} :: SoulCustom)

return shuffle
