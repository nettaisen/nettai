--!strict
-- BN5's computer-navi AI as Dark MegaMan runs it (0x0802BA14; the same code serves its other users, BN5's story navis
-- and its Liberation Missions, from 0x0802B4AC and 0x0802B800): it plays the tactics of the side across from it (the
-- player whose Chaos Unison failed: their computer-navi data, the battle's `tactic`s), entry by entry. An entry is a
-- chip, which it gets in place for (./place) and uses, or a pattern: a place by its target and a run of chips. Three
-- times in four it plays the first entry; otherwise, or with nothing to play, it steps into the row of an enemy and
-- fires its buster (three shots, ./buster), twice. Its state is the navi's (the system's `navi_state`, AIData +0xF0 on).

local data = require("./data")
local panels = require("./panels")
local place = require("./place")

local ai = {}

export type State = {
    -- AIData +0x83 and +0x88 (0x08104210): its time in the battle has started, and the ticks left of it.
    timed: boolean,
    lifetime: number,
    -- +0xF0: on (what a player navi under the AI's control reads; Dark MegaMan's is never read).
    active: boolean,
    -- +0xF1: playing a chip (4) or deciding (0); +0xF2 the decision's step; +0xF3 a step's own.
    mode: number,
    step: number,
    sub: number,
    -- +0xF4: decisions in a row that played nothing.
    misses: number,
    -- +0xF5: where a pattern's search for a target starts; +0xFC: where a chip's does.
    target_index: number,
    next_target: number,
    -- +0xF9: buster shots fired between steps.
    shots: number,
    -- +0xFB: the next chip of a pattern.
    pattern_chip: number,
    -- +0x114: the entry it plays: a chip, or a pattern (its number from 1).
    entry_chip: Chip?,
    entry_pattern: number,
    -- +0x120: when its dodging began (the battle's ticks).
    since: number,
    -- +0xF6, +0xF8, +0x110, +0x112: how long the side's front stood occupied and its area held broken panels, which
    -- would make it pick an entry for them (0x0802C012, 0x0802C026). Their only writer, 0x0802BF4E, nothing calls:
    -- they stay 0.
    hole_count: number,
    front_count: number,
    front_ticks: number,
    hole_ticks: number,
    -- +0x130 on: getting in place for a chip (./place).
    place_step: number,
    place_step_init: number,
    place_busy: boolean,
    place_ready: boolean,
    place_chip: Chip?,
    place_target: Object?,
}

-- The navi state's layout (the system's `navi_state`).
ai.layout = {
    timed = "bool",
    lifetime = "u16",
    active = "bool",
    mode = "u8",
    step = "u8",
    sub = "u8",
    misses = "u8",
    target_index = "u8",
    next_target = "u8",
    shots = "u8",
    pattern_chip = "u8",
    entry_chip = "chip",
    entry_pattern = "u8",
    since = "u32",
    hole_count = "u8",
    front_count = "u8",
    front_ticks = "u16",
    hole_ticks = "u16",
    place_step = "u8",
    place_step_init = "u8",
    place_busy = "bool",
    place_ready = "bool",
    place_chip = "chip",
    place_target = "object",
}

-- What it fires between moves, its buster (weapon routine 0x3E: three shots, ./buster).
local buster: Weapon = require("./buster").weapon

-- ---- Targets --------------------------------------------------------------------------------------------------

-- 0x0802BDE0: target `i` of the other side: 0 its first field-object slot, 1 to 4 its alive actors' slots.
local function target_at(navi: Object, i: number): Object?
    local other = bit32.bxor(navi.alliance, 1)
    if i == 0 then
        return field.object_slot(other, 1)
    end
    return battle.alive_actor_slot(other, i)
end

-- 0x0802BD92: the first target from `start` on (wrapping after 4), and its index; none: nil and `start`.
local function target_from(navi: Object, start: number): (Object?, number)
    local i = start
    while true do
        local t = target_at(navi, i)
        if t ~= nil then
            return t, i
        end
        i += 1
        if i > 4 then
            i = 0
        end
        if i == start then
            return nil, i
        end
    end
end
ai.target_from = target_from

-- 0x0802BDB4: the same from the chip's search index, which it keeps.
local function next_chip_target(navi: Object, s: State): Object?
    local t, i = target_from(navi, s.next_target)
    s.next_target = i
    return t
end

-- 0x0802BE06: the chip's search starts one further on (./place's, which its searches call too).
ai.skip_target = place.skip_target

-- ---- Tactics ------------------------------------------------------------------------------------------------

-- The side whose tactics it plays (the other side's: 0x0802BEEA, 0x0802BEFE, 0x0802BF1C, 0x0802C0DC).
local function tactics_side(navi: Object): number
    return bit32.bxor(navi.alliance, 1)
end

-- 0x0802BF1C: the played entry goes last; nothing missed since.
local function turn(navi: Object, s: State)
    battle.turn_tactics(tactics_side(navi))
    s.misses = 0
end

-- 0x0802C0DC: the first entry changes places with one of the others at random (a draw even with none).
local function shuffle(navi: Object)
    local side = tactics_side(navi)
    local r = battle.rng_positive()
    local n = battle.tactic_count(side) - 1
    if n > 0 then
        battle.swap_tactics(side, r % n + 2)
    end
end

-- 0x0802BE4E and 0x0802BE9A (the navi's own side's tactics), 0x0802BE54 and 0x0802BEA0 (the other side's): the
-- first of side `side`'s first `n` entries that is a chip whose record's byte 14 has `bit` comes first (it and the
-- first change places); none before an empty place: nil.
local function pick_by(side: number, bit: number, n: number): Chip?
    for i = 1, n do
        local e = battle.tactic(side, i)
        if e == nil then
            return nil
        end
        if type(e) ~= "number" then
            local chip = e :: Chip
            if bit32.band(data.chips[chip.id].picks, bit) ~= 0 then
                battle.swap_tactics(side, i)
                return chip
            end
        end
    end
    return nil
end

-- 0x0802C012, 0x0802C026: the side's front stood occupied, and its area held broken panels, (seconds) times (how
-- many).
local function front_pressure(s: State): number
    return (s.front_ticks // 60) * s.front_count
end
local function hole_pressure(s: State): number
    return (s.hole_ticks // 60) * s.hole_count
end

-- ---- Acting -------------------------------------------------------------------------------------------------

-- 0x0802C094: `chip` as its attack (its record's damage, hit parameter, lockout, element), and the chip's use (a
-- `set_attack` of kind 2): none of a hand's machinery.
function ai.use_chip(navi: Object, chip: Chip)
    navi:load_chip_attack(chip)
    navi:start_chip_attack(chip, 2)
end

-- `sub_80116F6`: a step to (x, y), facing the target the pattern search names.
local function step_to(navi: Object, s: State, x: number, y: number)
    local face = target_from(navi, s.target_index)
    navi:start_move_to(x, y, navi:move_lag(), face)
end

-- ---- Decisions (+0xF1 = 0) ----------------------------------------------------------------------------------

local function to_buster(s: State)
    s.step, s.sub, s.shots = 8, 0, 0
end

-- 0x0802BA64
local function decide(navi: Object, s: State)
    if not navi:chips_enabled() then
        return to_buster(s)
    end
    if front_pressure(s) >= 50 then
        -- (Its pick, 0x0802BA78, calls 0x081BC8AC, which is data, not code. The counters stay 0, so it never runs.)
        error("the computer navi's front pressure reached 50: its pick (0x0802BA78) calls 0x081BC8AC, which is data")
    end
    if hole_pressure(s) >= 50 then
        -- (Its own side's tactics, where the rest of its decision plays the other side's.)
        local chip = pick_by(navi.alliance, 2, 42)
        if chip then
            ai.use_chip(navi, chip)
            turn(navi, s)
            s.hole_ticks = 0
            return
        end
    end
    if s.misses >= 5 then
        shuffle(navi)
    end
    if bit32.band(battle.rng_positive(), 15) >= 12 then
        return to_buster(s)
    end
    local e = battle.tactic(tactics_side(navi), 1)
    if e == nil then
        s.misses += 1
        return to_buster(s)
    end
    if type(e) == "number" then
        s.entry_chip, s.entry_pattern = nil, e
        s.step, s.pattern_chip = 0x10, 0
        return
    end
    local chip = e :: Chip
    s.entry_chip, s.entry_pattern = chip, 0
    if data.chips[chip.id].approach and s.misses == 0 then
        s.step = 0x18
        return
    end
    s.mode, s.sub = 4, 0
end

-- 0x0802BB20: a breath; one time in two it turns its activity off.
local function rest(navi: Object, s: State)
    s.step = 0
    if bit32.band(battle.rng_positive(), 1) ~= 0 then
        s.active = false
    end
end

-- 0x0802BB34: into an enemy's row (0x0802BB50), its buster (0x0802BB7A); twice, then a decision.
local function buster_run(navi: Object, s: State)
    if s.sub == 0 then
        s.sub = 4
        local x, y = panels.row_of_nearest_enemy(navi)
        if x ~= nil then
            step_to(navi, s, x :: number, y :: number)
        end
    elseif s.sub == 4 then
        -- (Its console's chip window closes, `sub_801DACC(0x40)`, when its side is the local one: the HUD's.)
        navi:start_weapon(buster, 1)
        s.shots += 1
        if s.shots < 2 then
            s.sub = 0
        else
            s.step = 0
        end
    else
        error(string.format("the computer navi's buster run's step %#x reads past its table (0x0802BB48)", s.sub))
    end
end

-- 0x0802BBAC: up and down for 50 ticks, into an enemy's row, then the charged shot. (No decision of Dark MegaMan's
-- leads here.)
local function dodge(navi: Object, s: State)
    if s.sub == 0 then
        s.since = battle.battle_time()
        s.sub = 4
    elseif s.sub == 4 then
        if battle.battle_time() - s.since < 50 then
            local x, y = panels.random_up_down(navi)
            if x ~= nil then
                step_to(navi, s, x :: number, y :: number)
            end
            return
        end
        local x, y = panels.row_of_nearest_enemy(navi)
        if x ~= nil then
            step_to(navi, s, x :: number, y :: number)
        end
        s.sub = 8
    elseif s.sub == 8 then
        navi:reset_charge()
        local shot = navi.charge_shot_weapon
        if shot == nil then
            error("the computer navi's dodge ends in its charged shot (sub_80117A4), and it has none")
        end
        navi:start_weapon(shot :: Weapon, 1)
        s.step = 0
    else
        error(string.format("the computer navi's dodge's step %#x reads past its table (0x0802BBC0)", s.sub))
    end
end

-- 0x0802BC48: to its pattern's place by its target (or, with no target or a place it can't step onto, anywhere of
-- its own area, nothing played).
local function pattern_place(navi: Object, s: State)
    local dx, dy = battle.tactic_pattern(tactics_side(navi), s.entry_pattern)
    local t, i = target_from(navi, s.target_index)
    if t ~= nil then
        s.target_index = i
        local x = t.panel_x + dx * panels.enemy_direction(navi)
        local y = t.panel_y + dy
        if navi:can_step(x, y) then
            step_to(navi, s, x, y)
            s.step = 0x14
            return
        end
    end
    s.step = 0
    s.misses += 1
    place.skip_target(s)
    local x, y = panels.random_own(navi)
    step_to(navi, s, x, y)
end

-- 0x0802BCD6: its pattern's chips, one a decision; past the last, the pattern goes last.
local function pattern_chips(navi: Object, s: State)
    local _, _, chips = battle.tactic_pattern(tactics_side(navi), s.entry_pattern)
    s.pattern_chip += 1
    local chip = chips[s.pattern_chip]
    if chip == nil then
        turn(navi, s)
        s.step = 4
        return
    end
    ai.use_chip(navi, chip)
end

-- 0x0802BD08: a chip it walks up to first: anywhere of its own area, this time.
local function reposition(navi: Object, s: State)
    s.step = 0
    s.misses += 1
    local x, y = panels.random_own(navi)
    step_to(navi, s, x, y)
end

-- 0x0802BA34
local function deciding(navi: Object, s: State)
    local st = s.step
    if st == 0 then
        decide(navi, s)
    elseif st == 4 then
        rest(navi, s)
    elseif st == 8 then
        buster_run(navi, s)
    elseif st == 0xC then
        dodge(navi, s)
    elseif st == 0x10 then
        pattern_place(navi, s)
    elseif st == 0x14 then
        pattern_chips(navi, s)
    elseif st == 0x18 then
        reposition(navi, s)
    else
        error(string.format("the computer navi's decision step %#x reads past its table (0x0802BA48)", st))
    end
end

-- ---- Playing a chip (+0xF1 = 4) --------------------------------------------------------------------------------

-- 0x0802BD34: a target (none: back to deciding), in place for the chip (./place), the chip; it goes last.
local function playing(navi: Object, s: State)
    local sub = s.sub
    if sub == 0 then
        local t = next_chip_target(navi, s)
        if t == nil then
            s.mode = 0
            return
        end
        place.start(s, s.entry_chip :: Chip, t)
        s.sub = 4
    elseif sub == 4 then
        if not place.step(navi, s) then
            s.sub = 8
        end
    elseif sub == 8 then
        ai.use_chip(navi, s.entry_chip :: Chip)
        turn(navi, s)
        s.mode = 0
    end
end

-- 0x0802BA14: one decision of the AI, from the navi's idle.
function ai.run(navi: Object, s: State)
    if s.mode == 0 then
        deciding(navi, s)
    elseif s.mode == 4 then
        playing(navi, s)
    else
        error(string.format("the computer navi's mode %#x reads past its table (0x0802BA2C)", s.mode))
    end
end

return ai
