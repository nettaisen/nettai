--!strict
-- EXE4's auto battle AI (0x0800E4F8), as a netbattle runs it: for a dark MegaMan in his dark soul, from his idle in
-- place of the pad's step (rules/souls/dark). EXE5's AI comes from it (its 0x0802B4AC family, a navi playing its own
-- side's auto battle data, rules/auto_battle there), and plays the same way: entry by entry of the navi's own side's
-- data (./block), an entry a chip, which it gets in place for (./place) and uses, or a pattern, which the game never
-- plays (`pattern_place`). Three times in four it plays the first entry; otherwise, or with nothing to play, it fires
-- its buster from an enemy's row, twice, or dodges up and down with a charge showing and fires its charged shot. A side
-- that has long lost panels of its half, or stood long on broken or poison ones, plays the first chip that answers
-- that (PanlGrab's kind, Snctuary's) ahead of its turn.
--
-- Its state is 64 bytes a side at 0x02035A70 (AIData +0x50 points there), which the dark soul's status reset zeroes
-- (0x0800E18E) and any chip's use from idle does (0x080EED38); the chip's target search's index is AIData +0x17, and
-- the placing's state 16 bytes a side at 0x0203C030: the rules' state of the side.
--
-- What the port leaves as comments, nothing of a netbattle reaching it: the block's +0 (the AI on, which a story
-- navi's idle reads, 0x080EED48, and `rest` clears); the targets past the other player (`target_at`); a pattern's
-- place and its run of chips (`pattern_place`).

local block = require("./block")
local navi_list = require("../../lib/navi_list")
local panels = require("@exelib/auto_battle/panels")
local place = require("./place")

local ai = {}

export type State = {
    -- +1: playing a chip (4) or deciding (0); +2 the decision's step; +3 a step's own.
    mode: number,
    step: number,
    sub: number,
    -- +4: decisions in a row that played nothing.
    misses: number,
    -- +8 and +0x20: the panels of the side's half it doesn't hold, and the ticks there have been any.
    lost: number,
    lost_ticks: number,
    -- +6 and +0x22: the side's broken and poison panels, and the ticks there have been any.
    spoiled: number,
    spoiled_ticks: number,
    -- +9: buster shots fired between steps.
    shots: number,
    -- +0x24: the chip it plays.
    entry_chip: Chip?,
    -- +0x30: when its dodging began (the battle's ticks).
    since: number,
    -- AIData +0x17: where a chip's search for a target starts (not the block's: its zeroing leaves it).
    next_target: number,
    -- 0x0203C030 on: getting in place for a chip (./place).
    place_step: number,
    place_busy: boolean,
    place_ready: boolean,
    place_chip: Chip?,
    place_target: Object?,
}

ai.STATE = {
    mode = "u8",
    step = "u8",
    sub = "u8",
    misses = "u8",
    lost = "u8",
    lost_ticks = "u16",
    spoiled = "u8",
    spoiled_ticks = "u16",
    shots = "u8",
    entry_chip = "chip",
    since = "u32",
    next_target = "u8",
    place_step = "u8",
    place_busy = "bool",
    place_ready = "bool",
    place_chip = "chip",
    place_target = "object",
} :: StateSchema

-- The decision's steps (+2), its jump table's (0x0800E52C).
local DECIDE, REST, BUSTER_RUN, DODGE, PATTERN_PLACE, REPOSITION = 0, 4, 8, 0xC, 0x10, 0x18
-- The ticks of a dodge.
local DODGE_TICKS = 50
-- How long a side's loss weighs before a chip answers it: its seconds times its panels (0x0800E558, 0x0800E578).
local ANSWERED_AT = 50

-- ---- Targets --------------------------------------------------------------------------------------------------

-- 0x0800E884: target `i` of the other side: 0 the last of its three field-object slots (BattleState +0x70 and 12 a
-- side, where the registry starts at +0x68: a side's third object, where EXE5's AI reads its first); 1 its player's
-- navi (the local console's list, BattleState +0x44, by the other side's place in it); 2 to 4 that list's entries
-- from its second on (lib/navi_list: the other console's navi, whichever side that is, then none), which the
-- searches reach only with neither a third object nor the other side's navi: no fight's.
local function target_at(navi: Object, i: number): Object?
    local other = bit32.bxor(navi.alliance, 1)
    if i == 0 then
        return field.object_slot(other, 3)
    elseif i == 1 then
        return battle.player(other)
    end
    return navi_list.at(i - 1)
end

-- 0x0800E85C: the first target from the chip's search index on (wrapping after 4), which it keeps; none: nil.
local function next_chip_target(navi: Object, s: State): Object?
    local start = s.next_target
    local i = start
    repeat
        local t = target_at(navi, i)
        if t ~= nil then
            s.next_target = i
            return t
        end
        i = if i >= 4 then 0 else i + 1
    until i == start
    return nil
end

-- 0x0800E8B6: the chip's search starts one further on, of the two a netbattle has (five for side 0 in a story's
-- battle: a battle type under 70).
ai.skip_target = place.skip_target

-- ---- The auto battle data --------------------------------------------------------------------------------------

-- 0x0800E9B4: the played entry goes last; nothing missed since.
local function turn(navi: Object, s: State)
    block.turn(navi.alliance)
    s.misses = 0
end

-- 0x0800EB60: the first entry changes places with one of the others at random (a draw even with none).
local function shuffle(navi: Object)
    local side = navi.alliance
    local r = battle.rng_positive()
    local n = block.count(side) - 1
    if n > 0 then
        block.swap(side, r % n + 2)
    end
end

-- ---- Acting -------------------------------------------------------------------------------------------------

-- 0x0800EB1E: `chip` as its attack (its record's damage, hit parameter, lockout, element), and the chip's use (its
-- record's action): none of a hand's machinery.
function ai.use_chip(navi: Object, chip: Chip)
    navi:load_chip_attack(chip)
    navi:start_chip_attack(chip, 2)
end

-- 0x0800C9BA after 0x0800C208: a step to (x, y), the step's end lag its own.
local function step_to(navi: Object, x: number, y: number)
    navi:start_move_to(x, y, navi:move_lag())
end

-- ---- Decisions (+1 = 0) --------------------------------------------------------------------------------------

-- 0x0800E5EE: the buster's run or the dodge, by a draw.
local function to_buster(s: State)
    s.step = if bit32.band(battle.rng_positive(), 1) ~= 0 then BUSTER_RUN else DODGE
    s.sub, s.shots = 0, 0
end

-- 0x0800E554, 0x0800E574: a loss that has weighed long enough (0x0800EAA2, 0x0800EAB4: its seconds times its panels)
-- is answered by the first chip of the data that answers it (./block's `answer`), used where the navi stands and
-- turned last.
local function answer(navi: Object, s: State, ticks: number, count: number, loss: string, places: number): boolean
    if (ticks // 60) * count < ANSWERED_AT then
        return false
    end
    local chip = block.answer(navi.alliance, loss, places)
    if chip == nil then
        return false
    end
    ai.use_chip(navi, chip)
    turn(navi, s)
    return true
end

-- 0x0800E548
local function decide(navi: Object, s: State)
    if not navi:chips_enabled() then
        return to_buster(s)
    end
    -- (0x0800E910 looks through the data's first 30 places, 0x0800E952 through its 42.)
    if answer(navi, s, s.lost_ticks, s.lost, "area_lost", 30) then
        s.lost_ticks = 0
        return
    end
    if answer(navi, s, s.spoiled_ticks, s.spoiled, "panels_spoiled", block.PLACES) then
        s.spoiled_ticks = 0
        return
    end
    if s.misses >= 5 then
        shuffle(navi)
    end
    if bit32.band(battle.rng_positive(), 15) >= 12 then
        return to_buster(s)
    end
    local e = block.entry(navi.alliance, 1)
    if e == nil then
        s.misses += 1
        return to_buster(s)
    end
    if e == false then
        -- (0x0800E5AA has no test of a 0, EXE5's 0x0802BAC6: the place is a chip's, number 0.)
        error("the auto-battling navi's first place holds 0: the game plays its chip table's first record, which is no chip (0x0800E5C4)")
    end
    if type(e) == "number" then
        -- (0x0800E5BA: the pattern's chip count, +0x0B, starts at 0; no run of chips follows, `pattern_place`.)
        s.step = PATTERN_PLACE
        return
    end
    local chip = e :: Chip
    s.entry_chip = chip
    -- (The record's byte 9 bit 0, 0x0800E5CA: its `dimming` flag.)
    if table.find(chip.flags or {}, "dimming") ~= nil and s.misses == 0 then
        s.step = REPOSITION
        return
    end
    s.mode, s.sub = 4, 0
end

-- 0x0800E608: a breath, and a draw. (One time in two the draw clears the block's +0, the AI on: a story navi's.)
local function rest(s: State)
    s.step = DECIDE
    battle.rng_positive()
end

-- 0x0800E61C: into an enemy's row (0x0800E638), its buster (0x0800E656: the chip window goes, 0x08015B9E(0x10), and
-- the weapon of its B button fires, 0x0800C9D2); twice, then a decision.
local function buster_run(navi: Object, s: State)
    if s.sub == 0 then
        s.sub = 4
        local x, y = panels.row_of_nearest_enemy(navi)
        if x ~= nil then
            step_to(navi, x :: number, y :: number)
        end
    elseif s.sub == 4 then
        navi:hide_chip_window()
        local buster = navi.buster_weapon
        if buster == nil then
            error("the auto-battling navi's buster run fires its buster (0x0800C9D2), and it has none")
        end
        navi:start_weapon(buster :: Weapon, 1)
        s.shots += 1
        if s.shots < 2 then
            s.sub = 0
        else
            s.step = DECIDE
        end
    else
        error(string.format("the auto-battling navi's buster run's step %#x reads past its table (0x0800E630)", s.sub))
    end
end

-- 0x0800E67C: up and down for 50 ticks with a B charge showing (0x0800E69C: AIData +9 2, and its count, +0x32, the
-- ticks since; the next tick's input makes the level of the count, 0x0800BB66, against the charged shot's own time:
-- the 40 the AI writes for a full charge's, +0x36, the intake's reload of the charge times takes back before any
-- count is read against it, 0x0800DA16), into an enemy's row, then the charged shot (0x0800E6FA: the charge dropped,
-- the weapon of the navi's charged shot fired, 0x0800CA60).
local function dodge(navi: Object, s: State)
    if s.sub == 0 then
        s.since = battle.battle_time()
        navi.charge_source, navi.charge_counter = 2, 0
        s.sub = 4
    elseif s.sub == 4 then
        local ticks = battle.battle_time() - s.since
        navi.charge_counter = ticks
        if ticks < DODGE_TICKS then
            local x, y = panels.random_up_down(navi)
            if x ~= nil then
                step_to(navi, x :: number, y :: number)
            end
            return
        end
        local x, y = panels.row_of_nearest_enemy(navi)
        if x ~= nil then
            step_to(navi, x :: number, y :: number)
        end
        s.sub = 8
    elseif s.sub == 8 then
        navi:reset_charge()
        local shot = navi.charge_shot_weapon
        if shot == nil then
            error("the auto-battling navi's dodge ends in its charged shot (0x0800CA60), and it has none")
        end
        navi:start_weapon(shot :: Weapon, 1)
        s.step = DECIDE
    else
        error(string.format("the auto-battling navi's dodge's step %#x reads past its table (0x0800E690)", s.sub))
    end
end

-- 0x0800E70E: to its pattern's place by a target, as written; as run, never, as EXE5's: the step test (0x0800ADA8) is
-- called after the move lag's routine (0x0800C208) and before the place is taken back off the stack (0x0800E742 to
-- 0x0800E754), so what it tests is what that routine left: in r0 the lag (4), in r1 the address of the NaviStats
-- blocks (0x0203BEC0, which 0x0800D742 leaves there), no row of the field. The test fails for every place, whatever
-- the record: a pattern entry is a miss, the chip's target search moved on, and a step anywhere of its own area; its
-- record (its place, its chips, which the step 0x14 would use one a decision, 0x0800E784) shows in no battle.
local function pattern_place(navi: Object, s: State)
    s.step = DECIDE
    s.misses += 1
    place.skip_target(s)
    local x, y = panels.random_own(navi)
    step_to(navi, x, y)
end

-- 0x0800E7B8: a chip it walks up to first: anywhere of its own area, this time.
local function reposition(navi: Object, s: State)
    s.step = DECIDE
    s.misses += 1
    local x, y = panels.random_own(navi)
    step_to(navi, x, y)
end

-- 0x0800E518
local function deciding(navi: Object, s: State)
    local st = s.step
    if st == DECIDE then
        decide(navi, s)
    elseif st == REST then
        rest(s)
    elseif st == BUSTER_RUN then
        buster_run(navi, s)
    elseif st == DODGE then
        dodge(navi, s)
    elseif st == PATTERN_PLACE then
        pattern_place(navi, s)
    elseif st == REPOSITION then
        reposition(navi, s)
    else
        -- (0x14, a pattern's chips: `pattern_place` never leads there.)
        error(string.format("the auto-battling navi's decision step %#x is none a netbattle reaches (0x0800E52C)", st))
    end
end

-- ---- Playing a chip (+1 = 4) ---------------------------------------------------------------------------------

-- 0x0800E7D8: a target (none: a miss, back to deciding), then on the same tick in place for the chip (./place): still
-- busy, the next decision goes on; ready, the chip, which goes last; not ready, a miss.
local function playing(navi: Object, s: State)
    if s.sub == 0 then
        local t = next_chip_target(navi, s)
        if t == nil then
            s.misses += 1
            s.mode = 0
            return
        end
        place.start(s, s.entry_chip :: Chip, t)
        s.sub = 4
    end
    if place.step(navi, s) then
        return
    end
    if s.place_ready then
        ai.use_chip(navi, s.place_chip :: Chip)
        turn(navi, s)
    else
        s.misses += 1
    end
    s.mode = 0
end

-- 0x0800E4F8: one decision, from the navi's idle.
function ai.run(navi: Object, s: State)
    if s.mode == 0 then
        deciding(navi, s)
    elseif s.mode == 4 then
        playing(navi, s)
    else
        error(string.format("the auto-battling navi's mode %#x reads past its table (0x0800E510)", s.mode))
    end
end

-- 0x0800EA16: the panels of the side's half of the field (its three columns) that it doesn't hold.
local function lost_panels(side: number): number
    local n = 0
    for x = if side == 0 then 1 else 4, if side == 0 then 3 else 6 do
        for y = 1, 3 do
            local p = field.panel(x, y)
            if p ~= nil and p.alliance ~= side then
                n += 1
            end
        end
    end
    return n
end

-- 0x0800EA5E: the side's panels that are broken or poison (types 1 and 4).
local function spoiled_panels(side: number): number
    local n = 0
    for x = 1, 6 do
        for y = 1, 3 do
            local p = field.panel(x, y)
            if p ~= nil and p.alliance == side and (p.kind == "broken" or p.kind == "poison") then
                n += 1
            end
        end
    end
    return n
end

-- 0x0800E9E0, each tick of the navi's update but a dimmed one (0x080EAED2: every player navi's, whatever its soul;
-- the port counts in the dark soul, whose status reset zeroes the block): what the side has lost, and for how long.
function ai.tick(navi: Object, s: State)
    if battle.dimmed() then
        return
    end
    s.lost = lost_panels(navi.alliance)
    s.lost_ticks = if s.lost ~= 0 then s.lost_ticks + 1 else 0
    s.spoiled = spoiled_panels(navi.alliance)
    s.spoiled_ticks = if s.spoiled ~= 0 then s.spoiled_ticks + 1 else 0
end

-- 0x0800EAC6: the block zeroed: its decisions from the start, its counts gone. (The chip's target search's index and
-- the placing's state are no part of it.)
function ai.reset(s: State)
    s.mode, s.step, s.sub, s.misses = 0, 0, 0, 0
    s.lost, s.lost_ticks, s.spoiled, s.spoiled_ticks = 0, 0, 0, 0
    s.shots, s.entry_chip, s.since = 0, nil, 0
end

return ai
