--!strict
-- EXE5's auto battle data (0xE0 bytes a player at 0x02034C20; docs/design/exe5-map.md §15.9): what a navi in auto
-- battle plays of a player. A player's setup states it as their save holds it (`auto_battle_places`, the block's 42
-- places; `auto_battle_records`, its eight pattern records), and the consoles send it as a battle starts (0x0802C7BE:
-- `send`, into each side's `auto_battle_places` of the rules' state), which the AI (./ai) reads and turns as it plays
-- the entries. A navi in auto battle on the other side plays it (the Dark MegaMan a failed Chaos Unison brings), and
-- a side's own drive under DarkInvs its own.
--
-- A place (a halfword of the block) is empty (0xFFFF: `{}`), a chip, a pattern record by its number from 1 (bit 15
-- with its index), or `zero` (the halfword 0: chip 0, which no save the game wrote holds; the send keeps it as an
-- entry, which the AI never plays). A record is where to stand from the target (`dx` columns toward the navi's
-- enemies, `dy` rows), its five chip places (the chips it uses there, in order, to the first empty one; a `zero` one
-- played as chip 0) and its score as the game's learning keeps it. A list past its last place or record: empty
-- places, records nothing has written (0xFF throughout). The places' part is EXE4's too (@exelib/auto_battle/places:
-- the send, an entry, the swap and the turn); the records are EXE5's own.

local places = require("@exelib/auto_battle/places")

local block = {}

-- The block's places, how many of them (the first) the send shuffles apart from the rest, its records, and a record's
-- chip places.
block.PLACES = places.PLACES
block.FIRST = places.FIRST
block.RECORDS = places.RECORDS
block.RECORD_CHIPS = 5

-- The setup's and the state's types.
local CHIP_PLACE: StateSchema = { chip = "chip", zero = "bool" }
local RECORD: StateSchema = { dx = "i8", dy = "i8", chips = schema.list(CHIP_PLACE, block.RECORD_CHIPS), score = "u32" }

block.PLACE = places.PLACE
block.CHIP_PLACE = CHIP_PLACE
block.RECORD = RECORD

-- A place as the setup and the state hold it.
export type Place = { chip: Chip?, pattern: number, zero: boolean }
export type ChipPlace = { chip: Chip?, zero: boolean }
export type Record = { dx: number, dy: number, chips: { ChipPlace }, score: number }

-- The lists the game writes the places in (0x0802C692 to 0x0802C6E6).
block.LISTS = places.LISTS

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

-- `round_setup`, once for the round: both sides' data as their consoles send it as the battle starts (0x0802C7BE:
-- @exelib/auto_battle/places' send, 84 draws of the battle's RNG), into each side's state. (The EXE5 lab's recordings:
-- each sent block is its save's places sent from the fight's RNG2 84 draws back; exe5-compat's recordings state the
-- places as the saves hold them and the RNG2 the send starts from.)
function block.send()
    local sent = places.send({ rules.setup_of(0).auto_battle_places, rules.setup_of(1).auto_battle_places })
    for side = 0, 1 do
        rules.state_of(side).auto_battle_places = sent[side + 1]
    end
end

-- How many entries side `side`'s sent data has.
function block.count(side: number): number
    return #rules.state_of(side).auto_battle_places
end

-- The entry in place `i` (from 1) of side `side`'s sent data: its chip, its pattern's number (from 1), false for a 0,
-- nil for an empty place or one past the count.
function block.entry(side: number, i: number): (Chip | number | false)?
    return places.entry(rules.state_of(side).auto_battle_places, i)
end

-- 0x0802C0DC / 0x0802C0D6's swap: the first entry and the one in place `i` (from 1) change places.
function block.swap(side: number, i: number)
    places.swap(rules.state_of(side).auto_battle_places, i)
end

-- 0x0802BF1C's list part: the first entry goes last, the rest move up (`sub_8000EB6` packs the entries but the
-- emptied first; the count stays). With no entries the original loops 2^32 times.
function block.turn(side: number)
    local state = rules.state_of(side)
    state.auto_battle_places = places.turn(state.auto_battle_places)
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


-- `validate`'s part: each place one thing (a chip, a pattern, a 0, or empty), a pattern one of the eight records,
-- and no chip among the 42 places that the AI can't play (the original crashes when its navi in auto battle goes to
-- play one, and the game's own writer never puts one there; a record's chip place may hold one, as the game writes
-- them there and a record never plays).
function block.validate(side: number, problem: (p: SetupProblem) -> ())
    places.validate(rules.setup_of(side).auto_battle_places, problem)
end

return block
