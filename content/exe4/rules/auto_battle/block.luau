--!strict
-- EXE4's auto battle data (0xE0 bytes a player at 0x020358B0; docs/design/exe4-map.md §18 item 74): what the dark soul
-- of a dark MegaMan plays of his own player (./ai). A player's setup states its 42 places as their save holds them
-- (`auto_battle_places`: save 0x5064), and the consoles send them as a battle starts (0x0800F208 from 0x080088AE:
-- `send`, into each side's `auto_battle_places` of the rules' state), which the AI reads and turns as it plays the
-- entries. The places' code is EXE5's too (@exelib/auto_battle/places).
--
-- The block's eight pattern records (from its +0x58: where to stand from a target, a run of chips, a score) no setup
-- states: a pattern entry never plays (./ai's `pattern_place`), so nothing of a battle shows them.

local places = require("@exelib/auto_battle/places")

local block = {}

block.PLACES = places.PLACES
block.PLACE = places.PLACE

export type Place = { chip: Chip?, pattern: number, zero: boolean }

-- `round_setup`, once for the round: both sides' data as their consoles send it as the battle starts (0x0800F208: the
-- places' send, 84 draws of the battle's RNG), into each side's state. (The lab's recordings: each sent block is its
-- save's places sent from the fight's RNG2 84 draws back; exe4-compat's recordings state the places as the saves hold
-- them and the RNG2 the send starts from.)
function block.send()
    local sent = places.send({ rules.setup_of(0).auto_battle_places, rules.setup_of(1).auto_battle_places })
    for side = 0, 1 do
        rules.state_of(side).auto_battle_places = sent[side + 1]
    end
end

-- How many entries side `side`'s sent data has (the block's +0x54).
function block.count(side: number): number
    return #rules.state_of(side).auto_battle_places
end

-- 0x0800E994: the entry in place `i` (from 1) of side `side`'s sent data: its chip, its pattern's number (from 1),
-- false for a 0, nil for an empty place or one past the count.
function block.entry(side: number, i: number): (Chip | number | false)?
    return places.entry(rules.state_of(side).auto_battle_places, i)
end

-- 0x0800EB60's swap, and the picks' (0x0800E946, 0x0800E988): the first entry and the one in place `i` (from 1)
-- change places.
function block.swap(side: number, i: number)
    places.swap(rules.state_of(side).auto_battle_places, i)
end

-- 0x0800E9B4's list part: the first entry goes last, the rest move up (0x08000CFC packs the entries but the emptied
-- first; the count stays).
function block.turn(side: number)
    local state = rules.state_of(side)
    state.auto_battle_places = places.turn(state.auto_battle_places)
end

-- `validate`'s part: the places' (each one thing, a pattern one of the eight records, no chip the AI can't play:
-- its positioning class past the table at 0x0800F3C8), and no place holding 0: where EXE5's AI takes a first place
-- of 0 for nothing to play, EXE4's has no test of it (0x0800E5AA) and plays its chip table's record 0, which is no
-- chip (the game's own writer puts none there).
function block.validate(side: number, problem: (text: string, field: string?, entry: number?) -> ())
    local stated: { Place } = rules.setup_of(side).auto_battle_places
    places.validate(stated, problem)
    for i, p in stated do
        if p.zero then
            problem(
                string.format("place %d of the auto battle data holds 0: the original plays its chip table's first record there, which is no chip", i),
                "auto_battle_places",
                i
            )
        end
    end
end

return block
