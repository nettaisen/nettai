--!strict
-- EXE5's auto battle AI, in the two families a netbattle runs. Dark MegaMan's (0x0802BA14: `ai.run`) plays the auto
-- battle data of the side across from it (the player whose Chaos Unison failed: `battle.auto_battle_entry`);
-- the one EXE5's story navis and DarkInvs's drive run (0x0802B4AC: `ai.run_own`, a player under DarkInvs
-- driving itself) plays the navi's own side's, and faces, as it steps, the target whose search starts at the step's
-- column (where the other starts at the pattern's search place); its pick for a crowded front is real code. Entry by
-- entry: an entry is a chip, which it gets in place for (./place) and uses, or a pattern: a place by its target and a
-- run of chips, which the game never plays (the test of its place is of a panel that isn't one, `pattern_place`: a
-- pattern entry is a miss and a step). Three times in four it plays the first entry; otherwise, or with nothing to play
-- (no entry, or a first place holding 0), it steps into the row
-- of an enemy and fires its buster (three shots, ./buster), twice. Its state is AIData +0xF0 on (Dark MegaMan's the
-- rules' `navi_state`, a driven player's the rules' state of its side), but +0xF0 itself, the navi's
-- `in_auto_battle`.

local data = require("./data")
local panels = require("./panels")
local place = require("./place")

local ai = {}

export type State = {
    -- AIData +0x83 and +0x88 (0x08104210): its time in the battle has started, and the ticks left of it.
    timed: boolean,
    lifetime: number,
    -- (+0xF0, on, is the navi's `in_auto_battle`: what a player navi under the AI's control reads.)
    -- A player's drive (DarkInvs, the rules' state of its side): its timer when it last stepped, which a new drive
    -- sets above (the rules' bookkeeping: the game starts the state over at the drive's start, 0x0802C03A).
    drive_ticks: number,
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

-- What it fires between moves, its buster (weapon routine 0x3E: three shots, ./buster).
local buster: Weapon = require("./buster").weapon

-- The two families (`ai.run`, `ai.run_own`): whose auto battle data they play, and where the target a step faces is
-- searched from.
export type Family = { own: boolean }
local OTHER: Family = { own = false }
local OWN: Family = { own = true }

-- ---- Targets --------------------------------------------------------------------------------------------------

-- 0x0802BDE0: target `i` of the other side: 0 its first field-object slot, 1 to 4 its alive actors' slots. Past 4
-- (0x0802B4AC's family searches from a column) the words the game reads run on: past side 0's four actor slots, side
-- 1's; past side 1's, the field-object registry (side 0's three slots, then side 1's).
local function target_at(navi: Object, i: number): Object?
    local other = bit32.bxor(navi.alliance, 1)
    if i == 0 then
        return field.object_slot(other, 1)
    end
    local word = other * 4 + i - 1
    if word < 8 then
        return battle.alive_actor_slot(word // 4, word % 4 + 1)
    end
    word -= 8
    if word < 6 then
        return field.object_slot(word // 3, word % 3 + 1)
    end
    error(string.format("the auto-battling navi's target %d reads past the field-object registry (0x0802BDE0)", i))
end

-- 0x0802BD92: the first target from `start` on (wrapping after 4), and its index; none: nil and `start`. (From past 4
-- the wrap never comes back to `start`: with no target at all the game searches for good.)
local function target_from(navi: Object, start: number): (Object?, number)
    local i = start
    local looked = 0
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
        looked += 1
        if looked > 5 then
            error(string.format("the auto-battling navi's target search from %d finds nothing and never ends (0x0802BD92)", start))
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

-- ---- The auto battle data ------------------------------------------------------------------------------------

-- The side whose auto battle data it plays: the other side's (0x0802BEEA, 0x0802BEFE, 0x0802BF1C, 0x0802C0DC), or its own
-- (0x0802BEE6, 0x0802BEFA, 0x0802BF16, 0x0802C0D6).
local function auto_battle_side(navi: Object, fam: Family): number
    return if fam.own then navi.alliance else bit32.bxor(navi.alliance, 1)
end

-- 0x0802BF1C (0x0802BF16): the played entry goes last; nothing missed since.
local function turn(navi: Object, s: State, fam: Family)
    battle.auto_battle_turn(auto_battle_side(navi, fam))
    s.misses = 0
end

-- 0x0802C0DC (0x0802C0D6): the first entry changes places with one of the others at random (a draw even with none).
local function shuffle(navi: Object, fam: Family)
    local side = auto_battle_side(navi, fam)
    local r = battle.rng_positive()
    local n = battle.auto_battle_count(side) - 1
    if n > 0 then
        battle.auto_battle_swap(side, r % n + 2)
    end
end

-- 0x0802BE4E and 0x0802BE9A (the navi's own side's auto battle data), 0x0802BE54 and 0x0802BEA0 (the other side's): the
-- first of side `side`'s first `n` entries that is a chip whose record's byte 14 has `bit` comes first (it and the
-- first change places); none before an empty place: nil. (A place holding 0 is read as the chip table's first record,
-- chip 0's, whose byte 14 is 0: passed over, as a pattern is.)
local function pick_by(side: number, bit: number, n: number): Chip?
    for i = 1, n do
        local e = battle.auto_battle_entry(side, i)
        if e == nil then
            return nil
        end
        if e ~= false and type(e) ~= "number" then
            local chip = e :: Chip
            if bit32.band(data.chips[chip.id].picks, bit) ~= 0 then
                battle.auto_battle_swap(side, i)
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

-- `sub_80116F6`: a step to (x, y), facing the target a search names: from the pattern's search place (0x0802BA14's),
-- or from the step's column (0x0802B4AC's, whose register holds it there).
local function step_to(navi: Object, s: State, x: number, y: number, fam: Family)
    local face = target_from(navi, if fam.own then x else s.target_index)
    navi:start_move_to(x, y, navi:move_lag(), face)
end

-- ---- Decisions (+0xF1 = 0) ----------------------------------------------------------------------------------

local function to_buster(s: State)
    s.step, s.sub, s.shots = 8, 0, 0
end

-- 0x0802BA64 (0x0802B4FC)
local function decide(navi: Object, s: State, fam: Family)
    if not navi:chips_enabled() then
        return to_buster(s)
    end
    if front_pressure(s) >= 50 then
        if not fam.own then
            -- (Its pick, 0x0802BA78, calls 0x081BC8AC, which is data, not code. The counters stay 0, so it never
            -- runs.)
            error("the auto-battling navi's front pressure reached 50: its pick (0x0802BA78) calls 0x081BC8AC, which is data")
        end
        -- (0x0802B510: the first of its own side's first 30 entries for a crowded front, 0x0802BE4E.)
        local chip = pick_by(navi.alliance, 1, 30)
        if chip then
            ai.use_chip(navi, chip)
            turn(navi, s, fam)
            s.front_ticks = 0
            return
        end
    end
    if hole_pressure(s) >= 50 then
        -- (Its own side's auto battle data, where the rest of Dark MegaMan's decision plays the other side's.)
        local chip = pick_by(navi.alliance, 2, 42)
        if chip then
            ai.use_chip(navi, chip)
            turn(navi, s, fam)
            s.hole_ticks = 0
            return
        end
    end
    if s.misses >= 5 then
        shuffle(navi, fam)
    end
    if bit32.band(battle.rng_positive(), 15) >= 12 then
        return to_buster(s)
    end
    -- (0x0802BAC6, 0x0802B55E: a first place holding 0 is tested before an empty one and taken the same way: nothing
    -- to play. But the send packs only the empty places away (0x0802C7BE), so a 0 is an entry, which comes first in
    -- its turn and stays there, each decision that reaches it a miss, until five in a row bring the shuffle above.
    -- No save the game wrote has one: a battle's end fills the places with chips' numbers, 0x0802C540.)
    local e = battle.auto_battle_entry(auto_battle_side(navi, fam), 1)
    if e == nil or e == false then
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

-- 0x0802BB20 (0x0802B5B8): a breath; one time in two it turns its activity off (+0xF0: `in_auto_battle`).
local function rest(navi: Object, s: State)
    s.step = 0
    if bit32.band(battle.rng_positive(), 1) ~= 0 then
        navi.in_auto_battle = false
    end
end

-- 0x0802BB34: into an enemy's row (0x0802BB50), its buster (0x0802BB7A); twice, then a decision.
local function buster_run(navi: Object, s: State, fam: Family)
    if s.sub == 0 then
        s.sub = 4
        local x, y = panels.row_of_nearest_enemy(navi)
        if x ~= nil then
            step_to(navi, s, x :: number, y :: number, fam)
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
        error(string.format("the auto-battling navi's buster run's step %#x reads past its table (0x0802BB48)", s.sub))
    end
end

-- 0x0802BBAC: up and down for 50 ticks, into an enemy's row, then the charged shot. (No decision of Dark MegaMan's
-- leads here.)
local function dodge(navi: Object, s: State, fam: Family)
    if s.sub == 0 then
        s.since = battle.battle_time()
        s.sub = 4
    elseif s.sub == 4 then
        if battle.battle_time() - s.since < 50 then
            local x, y = panels.random_up_down(navi)
            if x ~= nil then
                step_to(navi, s, x :: number, y :: number, fam)
            end
            return
        end
        local x, y = panels.row_of_nearest_enemy(navi)
        if x ~= nil then
            step_to(navi, s, x :: number, y :: number, fam)
        end
        s.sub = 8
    elseif s.sub == 8 then
        navi:reset_charge()
        local shot = navi.charge_shot_weapon
        if shot == nil then
            error("the auto-battling navi's dodge ends in its charged shot (sub_80117A4), and it has none")
        end
        navi:start_weapon(shot :: Weapon, 1)
        s.step = 0
    else
        error(string.format("the auto-battling navi's dodge's step %#x reads past its table (0x0802BBC0)", s.sub))
    end
end

-- 0x0802BC48 (0x0802B6DC): to its pattern's place by its target, as written; as run, never: anywhere of its own area,
-- nothing played, whatever the pattern. The step test (0x0800CAA0) is called after the move lag's routine (0x0800E0BE)
-- and before the place is taken back off the stack (0x0802BC7E to 0x0802BC8E), so what it tests is what that routine
-- left in r0 and r1, by the stats of the navi's side (NaviStats +0x29, the navi its player operates):
-- - MegaMan's (navi 0): the lag in both (0x0800E0D8 copies r1 into r0): the column 4 and the row 4, or 0 and 0 in
--   ShadowSoul;
-- - a team navi's (1 to 12): the lag in r0 (4; KnightMan's 10) and in r1 the address of that navi's row of the lag
--   table (0x0801D462 + 11 × its number, 0x0800E0E8), which is no row whatever the lag.
-- The field's rows are 1 to 3 (0x0800B33C), so the test fails for every navi at every place: a pattern entry is a
-- miss, the target's search moved on, and a step, and nothing of a pattern (its place, its chips, its score, the
-- record after it) shows in a battle. (The lab's chaos-ai/pattern-reached: a pattern at a free panel of his own
-- area, none of it played; chaos-ai/colonel-side: Dark MegaMan on the side of a player who operates Colonel.)
-- A team navi's stats are read by the Dark MegaMan that stands on its side; no team navi is itself in auto battle in
-- a netbattle: a dark chip fizzles for any navi but MegaMan and navi 23 (0x08010056: DarkInvs), and the last stand
-- is the player of AI index 0's alone (0x0802C17A: a team navi's is its number).
local function pattern_place(navi: Object, s: State, fam: Family)
    local dx, dy = battle.auto_battle_pattern(auto_battle_side(navi, fam), s.entry_pattern)
    local t, i = target_from(navi, s.target_index)
    if t ~= nil then
        s.target_index = i
        local x = t.panel_x + dx * panels.enemy_direction(navi)
        local y = t.panel_y + dy
        -- (Whether the side's navi is MegaMan: whether it has forms, as the rules ask it.)
        local lag = navi:move_lag()
        local megaman = battle.navi(navi.alliance).navi.forms ~= nil
        if megaman and navi:can_step(lag, lag) then
            step_to(navi, s, x, y, fam)
            s.step = 0x14
            return
        end
    end
    s.step = 0
    s.misses += 1
    place.skip_target(s)
    local x, y = panels.random_own(navi)
    step_to(navi, s, x, y, fam)
end

-- 0x0802BCD6: its pattern's chips, one a decision; at an empty place the pattern goes last. No battle gets here
-- (`pattern_place` above never takes the place). As written the read has no end but an empty place and takes whatever
-- else it finds for a chip's number (0x0802C094 with no test of it): a place holding 0, and past a record whose five
-- places all hold a chip the two halves of its score, the next record's `dx` and `dy` as one halfword, that record's
-- places and score, and so on to the block's last eight bytes (`battle.auto_battle_pattern_read`). A chip's number names
-- none here (content names chips, and the chip table's first record, chip 0, and its blank ones have no
-- definition): such a read is an error that says what the game would play.
local function pattern_chips(navi: Object, s: State, fam: Family)
    s.pattern_chip += 1
    local read = battle.auto_battle_pattern_read(auto_battle_side(navi, fam), s.entry_pattern, s.pattern_chip)
    if read == nil then
        turn(navi, s, fam)
        s.step = 4
        return
    end
    if type(read) == "number" then
        error(
            string.format(
                "the auto-battling navi's read of a pattern found the number %#06x where a chip's would be: the game plays its chip table's record of that number (0x0801E214 + 0x2C × %d; 0x0802BCD6, 0x0802C094)",
                read,
                read
            )
        )
    end
    ai.use_chip(navi, read :: Chip)
end

-- 0x0802BD08: a chip it walks up to first: anywhere of its own area, this time.
local function reposition(navi: Object, s: State, fam: Family)
    s.step = 0
    s.misses += 1
    local x, y = panels.random_own(navi)
    step_to(navi, s, x, y, fam)
end

-- 0x0802BA34 (0x0802B4CC)
local function deciding(navi: Object, s: State, fam: Family)
    local st = s.step
    if st == 0 then
        decide(navi, s, fam)
    elseif st == 4 then
        rest(navi, s)
    elseif st == 8 then
        buster_run(navi, s, fam)
    elseif st == 0xC then
        dodge(navi, s, fam)
    elseif st == 0x10 then
        pattern_place(navi, s, fam)
    elseif st == 0x14 then
        pattern_chips(navi, s, fam)
    elseif st == 0x18 then
        reposition(navi, s, fam)
    else
        error(string.format("the auto-battling navi's decision step %#x reads past its table (0x0802BA48)", st))
    end
end

-- ---- Playing a chip (+0xF1 = 4) --------------------------------------------------------------------------------

-- 0x0802BD34 (0x0802B7BE): a target (none: back to deciding), in place for the chip (./place), the chip; it goes last.
local function playing(navi: Object, s: State, fam: Family)
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
        turn(navi, s, fam)
        s.mode = 0
    end
end

local function run(navi: Object, s: State, fam: Family)
    if s.mode == 0 then
        deciding(navi, s, fam)
    elseif s.mode == 4 then
        playing(navi, s, fam)
    else
        error(string.format("the auto-battling navi's mode %#x reads past its table (0x0802BA2C)", s.mode))
    end
end

-- 0x0802BA14: one decision of Dark MegaMan's AI, from the navi's idle (the other side's auto battle data).
function ai.run(navi: Object, s: State)
    run(navi, s, OTHER)
end

-- 0x0802B4AC: one decision of a navi driving itself on its own side's auto battle data (DarkInvs's drive), from its idle.
function ai.run_own(navi: Object, s: State)
    run(navi, s, OWN)
end

-- 0x0802C03A (0x0802BE3E's twin): AIData +0xF0 to +0x12F zeroed: the navi no longer in auto battle, its decisions'
-- state from the start (the time of AIData +0x83 and the placing of +0x130 on kept).
function ai.reset(navi: Object, s: State)
    navi.in_auto_battle = false
    s.mode, s.step, s.sub, s.misses = 0, 0, 0, 0
    s.target_index, s.next_target, s.shots, s.pattern_chip = 0, 0, 0, 0
    s.entry_chip, s.entry_pattern, s.since = nil, 0, 0
    s.hole_count, s.front_count, s.front_ticks, s.hole_ticks = 0, 0, 0, 0
end

return ai
