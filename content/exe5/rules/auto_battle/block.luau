--!strict
-- EXE5's auto battle data (0xE0 bytes a player at 0x02034C20; docs/design/exe5-map.md §15.9): what a navi in auto
-- battle plays of a player. A player's setup states it as their save holds it (`auto_battle_places`, the block's 42
-- places; `auto_battle_records`, its eight pattern records), and the console sends it as a battle starts (0x0802C7BE:
-- `send`, into the side's `auto_battle_places` of the rules' state), which the AI (./ai) reads and turns as it plays
-- the entries. A navi in auto battle on the other side plays it (the Dark MegaMan a failed Chaos Unison brings), and
-- a side's own drive under DarkInvs its own.
--
-- A place (a halfword of the block) is empty (0xFFFF: `{}`), a chip, a pattern record by its number from 1 (bit 15
-- with its index), or `zero` (the halfword 0: chip 0, which no save the game wrote holds; the send keeps it as an
-- entry, which the AI never plays). A record is where to stand from the target (`dx` columns toward the navi's
-- enemies, `dy` rows), its five chip places (the chips it uses there, in order, to the first empty one; a `zero` one
-- played as chip 0) and its score as the game's learning keeps it. A list past its last place or record: empty
-- places, records nothing has written (0xFF throughout).

local place = require("./place")

local block = {}

-- The block's places, how many of them (the first) the send shuffles apart from the rest, its records, and a record's
-- chip places.
block.PLACES = 42
block.FIRST = 3
block.RECORDS = 8
block.RECORD_CHIPS = 5

-- The setup's and the state's types.
local PLACE: StateSchema = { chip = "chip", pattern = "u8", zero = "bool" }
local CHIP_PLACE: StateSchema = { chip = "chip", zero = "bool" }
local RECORD: StateSchema = { dx = "i8", dy = "i8", chips = schema.list(CHIP_PLACE, block.RECORD_CHIPS), score = "u32" }

block.PLACE = PLACE
block.CHIP_PLACE = CHIP_PLACE
block.RECORD = RECORD

-- A place as the setup and the state hold it.
export type Place = { chip: Chip?, pattern: number, zero: boolean }
export type ChipPlace = { chip: Chip?, zero: boolean }
export type Record = { dx: number, dy: number, chips: { ChipPlace }, score: number }

-- The lists the game writes the places in (0x0802C692 to 0x0802C6E6): their names, first places (from 1) and sizes.
local LISTS: { { name: string, start: number, len: number } } = {
    { name = "first", start = 1, len = 3 },
    { name = "standard", start = 4, len = 24 },
    { name = "mega", start = 28, len = 5 },
    { name = "giga", start = 33, len = 1 },
    { name = "patterns", start = 34, len = 8 },
    { name = "program_advance", start = 42, len = 1 },
}

block.LISTS = LISTS

-- What the game's battle end writes of a player it has learned nothing of (0x0802C540), the records of a setup
-- that says nothing: every record zeros (its places are none: empty).
function block.nothing_learned(): { any }
    local records = {}
    for i = 1, block.RECORDS do
        local chips = {}
        for k = 1, block.RECORD_CHIPS do
            chips[k] = { zero = true }
        end
        records[i] = { dx = 0, dy = 0, chips = chips, score = 0 }
    end
    return records
end

local function empty(p: Place): boolean
    return p.chip == nil and p.pattern == 0 and not p.zero
end

-- A place copied out of a setup or a state.
local function copy(p: Place): Place
    return { chip = p.chip, pattern = p.pattern, zero = p.zero }
end

local EMPTY: Place = { chip = nil, pattern = 0, zero = false }

-- `round_setup`: side `side`'s data as its console sends it as the battle starts (0x0802C7BE), into the side's state:
-- the save's 42 places, the first three shuffled by three swaps and the other 39 by 39 (each swap two places drawn
-- from the battle's RNG, which stands in for the console's RNG2 at the link's start), the entries then packed to the
-- front (`sub_8000EB6`) and counted to the first empty place. A setup whose places are as a console sent them
-- (`auto_battle_sent`: a recording's) is the state as it is.
function block.send(side: number)
    local setup = rules.setup_of(side)
    local given = setup.auto_battle_places
    if setup.auto_battle_sent then
        local out: { Place } = {}
        for i, p in given do
            out[i] = copy(p)
        end
        rules.state_of(side).auto_battle_places = out
        return
    end
    local places: { Place } = {}
    for i = 1, block.PLACES do
        places[i] = if i <= #given then copy(given[i]) else EMPTY
    end
    local function shuffle(from: number, n: number, swaps: number)
        for _ = 1, swaps do
            local a = battle.rng_positive() % n
            local b = battle.rng_positive() % n
            places[from + a], places[from + b] = places[from + b], places[from + a]
        end
    end
    shuffle(1, block.FIRST, block.FIRST)
    shuffle(block.FIRST + 1, block.PLACES - block.FIRST, block.PLACES - block.FIRST)
    local sent: { Place } = {}
    for _, p in places do
        if not empty(p) then
            table.insert(sent, p)
        end
    end
    rules.state_of(side).auto_battle_places = sent
end

-- How many entries side `side`'s sent data has.
function block.count(side: number): number
    return #rules.state_of(side).auto_battle_places
end

-- The entry in place `i` (from 1) of side `side`'s sent data: its chip, its pattern's number (from 1), false for a 0,
-- nil for an empty place or one past the count.
function block.entry(side: number, i: number): (Chip | number | false)?
    local places = rules.state_of(side).auto_battle_places
    if i < 1 or i > #places then
        return nil
    end
    local p = places[i]
    if p.chip ~= nil then
        return p.chip
    elseif p.pattern > 0 then
        return p.pattern
    elseif p.zero then
        return false
    end
    return nil
end

-- 0x0802C0DC / 0x0802C0D6's swap: the first entry and the one in place `i` (from 1) change places.
function block.swap(side: number, i: number)
    local places = rules.state_of(side).auto_battle_places
    if i > #places then
        return
    end
    local first, other = copy(places[1]), copy(places[i])
    places[1] = other
    places[i] = first
end

-- 0x0802BF1C's list part: the first entry goes last, the rest move up (`sub_8000EB6` packs the entries but the
-- emptied first; the count stays). With no entries the original loops 2^32 times.
function block.turn(side: number)
    local places = rules.state_of(side).auto_battle_places
    local n = #places
    if n == 0 then
        error("a navi in auto battle turns auto battle data with no entries (0x0802BF1C: sub_8000EB6 counts 2^32 entries)")
    end
    local first = copy(places[1])
    local packed: { Place } = {}
    for i = 2, n do
        local p = copy(places[i])
        if not empty(p) then
            table.insert(packed, p)
        end
    end
    while #packed < n - 1 do
        table.insert(packed, EMPTY)
    end
    table.insert(packed, first)
    rules.state_of(side).auto_battle_places = packed
end

-- Record `r` (from 1) of side `side`'s data: past the setup's, one nothing has written (0xFF throughout).
local function record(side: number, r: number): Record?
    local records = rules.setup_of(side).auto_battle_records
    if r > #records then
        return nil
    end
    return records[r] :: any
end

-- Pattern `i`'s (from 1) place from its target: its record's `dx` and `dy`. Past the eighth record, an error: the
-- block has eight (0x0802BEFE reads past them).
function block.pattern(side: number, i: number): (number, number)
    if i < 1 or i > block.RECORDS then
        error(string.format("side %d's auto battle data has no pattern %d: the block has eight records (0x0802BEFE reads past them)", side, i))
    end
    local r = record(side, i)
    if r == nil then
        return -1, -1
    end
    return r.dx, r.dy
end

-- 0x0802BCD6: the `k`th halfword (from 1) the AI reads of pattern `i`'s (from 1) run, the one after its record's `dx`
-- and `dy` being the first: a chip place's chip, nil at an empty place (the run's end), else the halfword as a number,
-- which the game takes for a chip's number (0x0802C094). The read has no end but a halfword of 0xFFFF, so past a
-- record's five chip places it goes on: the two halves of its score, the next record's `dx` and `dy` as one halfword
-- (`dx` its low byte), that record's places and score, and so through the records to the block's last eight bytes,
-- which are 0xFF.
function block.pattern_read(side: number, i: number, k: number): (Chip | number)?
    -- A record's halfwords: its place, its chip places, its score's two halves.
    local per = 3 + block.RECORD_CHIPS
    local at = (i - 1) * per + k
    local r, field = at // per + 1, at % per
    if r > block.RECORDS then
        return nil
    end
    local function number(n: number): number?
        return if n == 0xFFFF then nil else n
    end
    local rec = record(side, r)
    if field == 0 then
        if rec == nil then
            return nil
        end
        return number(bit32.band(rec.dx, 0xFF) + bit32.band(rec.dy, 0xFF) * 0x100)
    elseif field <= block.RECORD_CHIPS then
        if rec == nil or field > #rec.chips then
            return nil
        end
        local c = rec.chips[field]
        if c.chip ~= nil then
            return c.chip
        elseif c.zero then
            return 0
        end
        return nil
    elseif field == block.RECORD_CHIPS + 1 then
        return number(if rec == nil then 0xFFFF else bit32.band(rec.score, 0xFFFF))
    end
    return number(if rec == nil then 0xFFFF else bit32.rshift(rec.score, 16))
end

-- The chips the AI can't get a navi in place for, by id, each with why (./place).
local UNPLAYABLE = place.unplayable()

-- `validate`'s part: each place one thing (a chip, a pattern, a 0, or empty), a pattern one of the eight records,
-- and no chip among the 42 places that the AI can't play (the original crashes when its navi in auto battle goes to
-- play one, and the game's own writer never puts one there; a record's chip place may hold one, as the game writes
-- them there and a record never plays).
function block.validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local places: { Place } = rules.setup_of(side).auto_battle_places
    for i, p in places do
        local list = block.LISTS[#block.LISTS]
        for _, l in block.LISTS do
            if i >= l.start and i < l.start + l.len then
                list = l
            end
        end
        local where = string.format("place %d of the auto battle data (`%s`, entry %d)", i, list.name, i - list.start + 1)
        local things = (if p.chip ~= nil then 1 else 0) + (if p.pattern > 0 then 1 else 0) + (if p.zero then 1 else 0)
        if things > 1 then
            problem(string.format("%s is a chip, a pattern or a 0, not more", where), "auto_battle_places", i)
        end
        if p.pattern > block.RECORDS then
            problem(
                string.format("place %d of the auto battle data names pattern %d; it has %d pattern records", i, p.pattern, block.RECORDS),
                "auto_battle_places",
                i
            )
        end
        local chip: Chip? = p.chip
        if chip ~= nil then
            local why = UNPLAYABLE[chip.id :: string]
            if why ~= nil then
                problem(string.format("%s holds %s: %s", where, battle.name(chip), why), "auto_battle_places", i)
            end
        end
    end
end

return block
