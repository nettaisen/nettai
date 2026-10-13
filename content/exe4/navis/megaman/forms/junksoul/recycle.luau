--!strict
-- JunkSoul's custom screen (0x0801DFBC, as a screen opens in the soul, 0x0801DD0C: its form's `custom`): two of the
-- chips sent this battle (rules/sent_chips: this console's list, either side's hands) are offered again in slots 8
-- and 9, in their codes (flagged 0x11: the screen's +0x30 and +0x32, each the chip's place in the list). The list is
-- shuffled first (0x08000B4A: 64 swaps of two of its entries, by RNG1's draws); then each is the first of the list
-- (but the one offered already) of a class a draw of RNG1 picks (0x0801E040). One chip sent is offered alone, and
-- none offers nothing (no shuffle, no draw). A chip picked there, or given up for a soul, leaves the list as OK
-- builds the hand (0x0801EFBA zeroes its entry, 0x0801EFE8 closes the list up) and isn't listed again: its hand
-- entry is marked 0x02 (0x0801EF4C answers the slot's flag 0x10 with Z clear, 0x0801F0B8 adds 2 to its flags), and
-- the hand's picks leave a marked entry out (0x0801F118), so it is never sent twice.
--
-- (The screen skips this while it deals dark chips, 0x0801DD08, which it deals no navi in a soul in a netbattle: in
-- a soul the emotion it reads is the soul's, 0x0800F4BA.)

local sent_chips = require("../../../../rules/sent_chips")

local recycle = {}

-- What the screen keeps (the rules' state): the list's entries it offers again (from 1), slot 8's then slot 9's.
type Screen = { offered_again: { number } }

-- The slots of the offer.
local SLOTS: { number } = { 8, 9 }
-- A chip offered again: its hand entry's mark (0x0801F0B8), and no pick of the hand's (0x0801F118). (The cursor on
-- one shades nothing, a dark chip or not: the hover reads the slot's flag 0x20, the dark offer's, 0x0801E4DE.)
local OFFERED_AGAIN: Offer = { marks = 0x02, listed = false }
-- The shuffle's swaps (0x0801DFCE).
local SWAPS = 64
-- The class a draw's low three bits pick (0x0801E0A4; with the navi's HP at half its maximum or less, 0x0801E0AC).
local HEALTHY: { string } = { "standard", "standard", "standard", "standard", "mega", "mega", "mega", "giga" }
local HURT: { string } = { "standard", "standard", "standard", "mega", "mega", "mega", "giga", "giga" }
-- A class with none listed gives way to the one below it, the standard chips to the Giga chips (0x0801E058).
local BELOW: { [string]: string } = { giga = "mega", mega = "standard", standard = "giga" }

-- 0x0801E040: a draw's class, or the first below it with a chip left.
local function draw_class(side: number, classes: { string }, left: { [string]: number }): string
    local class = classes[bit32.band(battle.console_rng_positive(side), 7) + 1]
    while left[class] == 0 do
        class = BELOW[class]
    end
    return class
end

-- 0x0801E064: the list's first chip of `class`, but entry `taken`.
local function first_of(sent: { FolderChip }, class: string, taken: number?): number
    for i, c in sent do
        if i ~= taken and c.chip.class == class then
            return i
        end
    end
    error("JunkSoul's offer counted a chip of a class the list hasn't")
end

-- 0x0801DFBC.
local function deal(side: number, s: Screen)
    s.offered_again = {}
    local sent = sent_chips.of(side)
    local n = #sent
    if n == 0 then
        return
    end
    for _ = 1, SWAPS do
        local a = battle.console_rng_positive(side) % n + 1
        local b = battle.console_rng_positive(side) % n + 1
        sent[a], sent[b] = sent[b], sent[a]
    end
    sent_chips.set(side, sent)
    -- 0x0801DFDA: how many of each class (a folder's chips are of these three).
    local left: { [string]: number } = { standard = 0, mega = 0, giga = 0 }
    for _, c in sent do
        local class: string = c.chip.class
        left[class] += 1
    end
    local navi = battle.player(side)
    local classes = if navi ~= nil and navi.hp * 2 <= navi.max_hp then HURT else HEALTHY
    local class = draw_class(side, classes, left)
    left[class] -= 1
    local first = first_of(sent, class, nil)
    custom.offer(side, SLOTS[1], sent[first].chip, sent[first].code, OFFERED_AGAIN)
    if n == 1 then
        s.offered_again = { first }
        return
    end
    local second = first_of(sent, draw_class(side, classes, left), first)
    custom.offer(side, SLOTS[2], sent[second].chip, sent[second].code, OFFERED_AGAIN)
    s.offered_again = { first, second }
end

-- 0x0801F0FA and 0x0801F074 through 0x0801EF7C's flag 0x10, then 0x0801EFE8: as OK builds the hand, the offered chips
-- picked (one given up for a soul stays picked) leave the list.
local function confirmed(side: number, s: Screen)
    local gone: { [number]: boolean } = {}
    local any = false
    for k = 1, #s.offered_again do
        local here = custom.slot(side, SLOTS[k])
        if here ~= nil and here.picked then
            gone[s.offered_again[k]] = true
            any = true
        end
    end
    if not any then
        return
    end
    local kept: { FolderChip } = {}
    for i, c in sent_chips.of(side) do
        if not gone[i] then
            kept[#kept + 1] = c
        end
    end
    sent_chips.set(side, kept)
end

recycle.custom = new.record("soul-custom", {
    state = { offered_again = schema.list("u8", 2) },
    deal = deal,
    confirmed = confirmed,
} :: SoulCustom)

return recycle
