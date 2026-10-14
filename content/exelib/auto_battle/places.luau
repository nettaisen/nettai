--!strict
-- The places of a player's auto battle data (EXE4's and EXE5's: a block of 0xE0 bytes a player, 42 halfword places
-- first), which a navi in auto battle plays: what the two games' code for them shares. A player's setup states the
-- places as their save holds them, and the consoles send them as a battle starts (`send`: EXE4's 0x0800F208, EXE5's
-- 0x0802C7BE, the same routine), after which the AI reads the first (`entry`), plays it and turns it last (`turn`),
-- or changes it for another (`swap`). Each game's rules keep the lists (their setup's and their state's
-- `auto_battle_places`) and hand them here (its rules/auto_battle/block).
--
-- A place (a halfword of the block) is empty (0xFFFF: `{}`), a chip, a pattern record by its number from 1 (bit 15
-- with its index), or `zero` (the halfword 0: chip 0, which no save the games wrote holds; the send keeps it as an
-- entry). A list past its last place: empty places.

local order = require("@exelib/order")

local places = {}

-- The block's places, how many of them (the first) the send shuffles apart from the rest, and its pattern records.
places.PLACES = 42
places.FIRST = 3
places.RECORDS = 8

-- A place as a setup and a state hold it.
local PLACE: StateSchema = { chip = "chip", pattern = "u8", zero = "bool" }
places.PLACE = PLACE
export type Place = { chip: Chip?, pattern: number, zero: boolean }

-- The lists the games write the places in (EXE5's 0x0802C692 to 0x0802C6E6): their names, first places (from 1) and
-- sizes, and `own_class` where the list holds chips of the class it is named for alone (`validate`; a tool reads it to
-- offer a place the chips it can hold): the mega, giga and program advance lists, which the games' writer fills with
-- the most used chips of that class (nettai's choice, docs/design/app-ui.md §2.27). The first and standard lists
-- take a chip of any class, though the writer puts only standard chips there.
local LISTS: { { name: string, start: number, len: number, own_class: boolean? } } = {
    { name = "first", start = 1, len = 3 },
    { name = "standard", start = 4, len = 24 },
    { name = "mega", start = 28, len = 5, own_class = true },
    { name = "giga", start = 33, len = 1, own_class = true },
    { name = "patterns", start = 34, len = 8 },
    { name = "program_advance", start = 42, len = 1, own_class = true },
}
places.LISTS = LISTS

local function empty(p: Place): boolean
    return p.chip == nil and p.pattern == 0 and not p.zero
end

-- A place copied out of a setup or a state.
local function copy(p: Place): Place
    return { chip = p.chip, pattern = p.pattern, zero = p.zero }
end

local EMPTY: Place = { chip = nil, pattern = 0, zero = false }

-- The send, once for a round: both sides' places as their setups state them (`given`, side 0's first), as their
-- consoles send them as the battle starts. Each console shuffles its own player's 42 places with the same draws, from
-- the RNG2 the link shares (the battle's RNG here), so both sides' places get the same order: the first three
-- places shuffled over three rounds and the other 39 over 39, two draws a round, 84 draws in all, after which the
-- battle's RNG goes on from where the send left it. The entries are then packed to the front and counted to the first
-- empty place. (A recording states the places as the saves hold them and the RNG2 the send starts from.)
function places.send(given: { { Place } }): { { Place } }
    local lists: { { Place } } = {}
    for side, stated in given do
        local p: { Place } = {}
        for i = 1, places.PLACES do
            p[i] = if i <= #stated then copy(stated[i]) else EMPTY
        end
        lists[side] = p
    end
    local function shuffle(from: number, n: number, swaps: number)
        local new = battle.shuffle_order(n, swaps)
        for _, p in lists do
            order.apply(p, from - 1, new)
        end
    end
    shuffle(1, places.FIRST, places.FIRST)
    shuffle(places.FIRST + 1, places.PLACES - places.FIRST, places.PLACES - places.FIRST)
    local out: { { Place } } = {}
    for side, p in lists do
        local sent: { Place } = {}
        for _, place in p do
            if not empty(place) then
                table.insert(sent, place)
            end
        end
        out[side] = sent
    end
    return out
end

-- The entry in place `i` (from 1) of sent data: its chip, its pattern's number (from 1), false for a 0, nil for an
-- empty place or one past the count.
function places.entry(sent: { Place }, i: number): (Chip | number | false)?
    if i < 1 or i > #sent then
        return nil
    end
    local p = sent[i]
    if p.chip ~= nil then
        return p.chip
    elseif p.pattern > 0 then
        return p.pattern
    elseif p.zero then
        return false
    end
    return nil
end

-- The first entry and the one in place `i` (from 1) change places.
function places.swap(sent: { Place }, i: number)
    if i > #sent then
        return
    end
    local first, other = copy(sent[1]), copy(sent[i])
    sent[1] = other
    sent[i] = first
end

-- The first entry goes last, the rest move up (the games' packing of the entries but the emptied first; the count
-- stays): the list after it. With no entries nothing of the games leads here.
function places.turn(sent: { Place }): { Place }
    local n = #sent
    if n == 0 then
        error("a navi in auto battle turns auto battle data with no entries (the original packs and counts 2^32 of them)")
    end
    local first = copy(sent[1])
    local packed: { Place } = {}
    for i = 2, n do
        local p = copy(sent[i])
        if not empty(p) then
            table.insert(packed, p)
        end
    end
    while #packed < n - 1 do
        table.insert(packed, EMPTY)
    end
    table.insert(packed, first)
    return packed
end

-- A game's `validate`'s part for a setup's places: each place one thing (a chip, a pattern, a 0, or empty), a
-- pattern one of the eight records, no chip among the 42 places that the game's AI can't play (the original
-- crashes when its navi in auto battle goes to play one, and the game's own writer never puts one there), and in a
-- list that holds the class it is named for alone (`LISTS`' `own_class`), no chip of another.
function places.validate(stated: { Place }, problem: (p: SetupProblem) -> ())
    for i, p in stated do
        local list = LISTS[#LISTS]
        for _, l in LISTS do
            if i >= l.start and i < l.start + l.len then
                list = l
            end
        end
        local where = string.format("place %d of the auto battle data (`%s`, entry %d)", i, list.name, i - list.start + 1)
        local things = (if p.chip ~= nil then 1 else 0) + (if p.pattern > 0 then 1 else 0) + (if p.zero then 1 else 0)
        if things > 1 then
            problem({
                kind = "invalid",
                field = "auto_battle_places",
                entry = i,
                text = string.format("%s is a chip, a pattern or a 0, not more", where),
            })
        end
        if p.pattern > places.RECORDS then
            problem({
                kind = "range",
                field = "auto_battle_places",
                entry = i,
                value = p.pattern,
                least = 0,
                most = places.RECORDS,
                text = string.format("place %d of the auto battle data names pattern %d; it has %d pattern records", i, p.pattern, places.RECORDS),
            })
        end
        local chip: Chip? = p.chip
        if chip ~= nil then
            if list.own_class and chip.class ~= list.name then
                problem({
                    kind = "chip_not_allowed",
                    field = "auto_battle_places",
                    entry = i,
                    chip = chip,
                    text = string.format(
                        "%s holds %s, a chip of class `%s`: the list holds `%s` chips alone",
                        where,
                        battle.name(chip),
                        chip.class or "none",
                        list.name
                    ),
                })
            end
            -- (A chip with no `auto_battle`, the game's rules/auto_battle/place: its positioning class is past the
            -- table.)
            if (chip :: any).auto_battle == nil then
                problem({
                    kind = "chip_not_allowed",
                    field = "auto_battle_places",
                    entry = i,
                    chip = chip,
                    text = string.format(
                        "%s holds %s: the original can't play it in auto battle (its positioning class is past the game's table of them: the game crashes)",
                        where,
                        battle.name(chip)
                    ),
                })
            end
        end
    end
end

return places
