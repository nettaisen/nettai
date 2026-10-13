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

-- The lists the game writes the places in, EXE5's (@exelib/auto_battle/places' `LISTS`; docs/design/exe4-map.md, "The
-- places' lists"): the routine that writes the block from the player's battles (0x0800EFA8, from 0x08006EAE; US Red
-- Sun's addresses) builds the 42 places and copies them to the block (0x0800F158): places 1 to 3 the second count
-- table's most counted standard chips (0x0800F0FA, its class 0 list 0x02034400), 4 to 27 the use counts' sixteen most
-- used standard chips by the repeat table 0x0800F1F8 (4, 4, 2, 2, then once each: 0x0800F10A, 0x0800F1C2), 28 to 32
-- the five most used mega chips (0x0800F118, class 1's list 0x02033400), 33 the most used giga chip (0x0800F128,
-- class 2's 0x02033800), 34 to 41 the patterns (0x0800F138: 0x0800F2DC), 42 the most used program advance
-- (0x0800F142, class 4's 0x02034000). A tool reads them as data (the build editor's auto battle).
block.LISTS = places.LISTS

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

-- 0x0800E910 (the data's first 30 places) and 0x0800E952 (its 42): the first chip, among the first `limit` places to
-- the first empty one, that answers the loss `loss` (its record's byte 14: bit 0 a loss of the side's area, PanlGrab's
-- kind; bit 1 its spoiled panels, Snctuary's kind: its `auto_battle.answers`), which changes places with the first
-- entry; none: nil.
function block.answer(side: number, loss: string, limit: number): Chip?
    local sent: { Place } = rules.state_of(side).auto_battle_places
    for i = 1, math.min(#sent, limit) do
        local chip: Chip? = sent[i].chip
        if chip ~= nil then
            local record: { answers: string? }? = (chip :: any).auto_battle
            if record ~= nil and record.answers == loss then
                places.swap(sent, i)
                return chip
            end
        end
    end
    return nil
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
function block.validate(side: number, problem: (p: SetupProblem) -> ())
    local stated: { Place } = rules.setup_of(side).auto_battle_places
    places.validate(stated, problem)
    for i, p in stated do
        if p.zero then
            problem({
                kind = "invalid",
                field = "auto_battle_places",
                entry = i,
                text = string.format("place %d of the auto battle data holds 0: the original plays its chip table's first record there, which is no chip", i),
            })
        end
    end
end

return block
