--!strict
-- A navi in EXE4's auto battle getting in place for a chip (0x0800F394, from the AI's chip play, ./ai): each chip's
-- record names how (byte 13, its positioning class: its `auto_battle.class`), and the class's routine runs once a
-- decision until it says it is no longer busy. Most classes have two steps: a search for a panel to use the chip
-- from, a step there, and the next decision's "in place" if a hit would reach the target (else a step anywhere of its
-- own area); the rest test a condition and say at once. The AI plays the chip only when the navi is ready. Its state
-- is the side's (16 bytes at 0x0203C030: the step +0, busy +2, ready +3, the chip +4, the target +8). EXE5's classes
-- come from these (rules/auto_battle/place there), a class of the same name the same search.

local data = require("./data")
local navi_list = require("../../lib/navi_list")
local navicust = require("../navicust")
local panels = require("@exelib/auto_battle/panels")

local place = {}

-- What a class's routine reads and writes of the AI's state (./ai's State: the place fields, and the chip's target
-- search's index).
type State = {
    place_step: number,
    place_busy: boolean,
    place_ready: boolean,
    place_chip: Chip?,
    place_target: Object?,
    next_target: number,
}

-- 0x0800F480: getting in place for `chip` against `target` starts (the step 0; ready cleared; busy as it was).
function place.start(s: State, chip: Chip, target: Object)
    s.place_chip, s.place_target = chip, target
    s.place_step = 0
    s.place_ready = false
end

-- 0x0800E8B6: the chip's next target search starts one further on, of the two targets a netbattle has (./ai's: the
-- other side's field object and its navi; five for side 0 in a story's battle, a battle type under 70).
function place.skip_target(s: State)
    s.next_target += 1
    if s.next_target > 1 then
        s.next_target = 0
    end
end

-- ---- Helpers ------------------------------------------------------------------------------------------------

local function chip_of(s: State): Chip
    return s.place_chip :: Chip
end

local function target_of(s: State): Object
    return s.place_target :: Object
end

-- What the AI reads of the chip's record: its own `auto_battle` (the rules' extension). None: the AI can't play it
-- (its positioning class is past the table at 0x0800F3C8: the original jumps wherever the memory after it says).
type AutoBattle = { class: string, range: string?, swings: { string }?, element_trap: boolean? }
local function record(chip: Chip): AutoBattle
    local r = (chip :: any).auto_battle :: AutoBattle?
    if r == nil then
        error(string.format(
            "the navi in auto battle plays %s, which the AI can't play (its positioning class is past the table at 0x0800F3C8)",
            chip.id :: string
        ))
    end
    return r
end

-- The chip's reach list or spot test (its `auto_battle.range`, which the classes that read it name).
local function range_of(chip: Chip): string
    local range = record(chip).range
    if range == nil then
        error(string.format("%s's positioning class %s reads a reach list, and it names none", chip.id :: string, record(chip).class))
    end
    return range
end

-- A reach list (the chip record's byte 23, data.ranges by name; 0x0801DAC8).
local function reach(range: string): { { number } }
    local list = data.ranges[range]
    if list == nil then
        error(string.format("the auto-battling navi's chip names reach list %s, which the AI's data hasn't", range))
    end
    return list
end

-- 0x08023106: one of the reach list's panels from the navi (dx toward its enemies) is the target's.
local function reaches(navi: Object, target: Object, range: string): boolean
    local dir = panels.enemy_direction(navi)
    for _, d in reach(range) do
        if navi.panel_x + d[1] * dir == target.panel_x and navi.panel_y + d[2] == target.panel_y then
            return true
        end
    end
    return false
end

-- 0x080230F0: a hit would reach the target: it is neither invisible, invulnerable nor flashing (its flags 0x20A).
local function hittable(target: Object): boolean
    return not (target:status("invisible") or target:status("invulnerable") or target:status("flashing"))
end

-- 0x0800C9BA after 0x0800C208: a step to (x, y), the step's end lag its own.
local function move(navi: Object, x: number, y: number)
    navi:start_move_to(x, y, navi:move_lag())
end

local function in_place(s: State)
    s.place_busy, s.place_ready = false, true
end

local function not_ready(s: State)
    s.place_busy, s.place_ready = false, false
end

-- 0x0800F734: a step anywhere of its own area.
local function wander(navi: Object)
    local x, y = panels.random_own(navi)
    move(navi, x, y)
end

-- A step to (x, y), the class's second step next decision.
local function go(navi: Object, s: State, x: number, y: number)
    s.place_step = 4
    move(navi, x, y)
end

-- The search found nothing: not ready, a step anywhere of its own area.
local function settle(navi: Object, s: State)
    not_ready(s)
    wander(navi)
end

-- The same, and the chip's next target search starts one further on.
local function give_up(navi: Object, s: State)
    place.skip_target(s)
    settle(navi, s)
end

-- A step to what a search found, or with nothing found `otherwise`.
local function go_or(navi: Object, s: State, x: number?, y: number?, otherwise: (navi: Object, s: State) -> ())
    if x == nil then
        return otherwise(navi, s)
    end
    go(navi, s, x, y :: number)
end

-- The panels' flags the searches read: one a body stands on (0x10), a cracked one (0x40), grass (0x400); any body
-- on it (0x0F800000); the other side's bodies a side looks for (0x0800FA7C and its copies: 0x04000000 for side 0,
-- 0x08000000 for 1); a body of the side's own or a neutral one (0x0800F834).
local STANDABLE = 0x10
local ANY_BODY = 0x0F800000
local ENEMY_BODIES: { number } = { 0x04000000, 0x08000000 }
local OWN_BODIES: { number } = { 0x01800000, 0x02800000 }

local function by_side<T>(pair: { T }, side: number): T
    return pair[bit32.band(side, 1) + 1]
end

local function packed(x: number, y: number): number
    return bit32.bor(x, bit32.lshift(y, 4))
end

local function unpacked(p: number): (number, number)
    return bit32.band(p, 7), bit32.rshift(p, 4)
end

-- The panel (x, y)'s type.
local function kind_at(x: number, y: number): PanelType?
    local p = field.panel(x, y)
    return if p then p.kind else nil
end

-- The panels meeting a side's conditions (`object_getPanelsFiltered` over a table by side): how many.
local function count_panels(conds: { { number } }, side: number): number
    local c = by_side(conds, side)
    return #panels.filtered(c[1], c[2])
end

-- Of the panels the navi may step onto, those `test` passes: one at random.
local function step_where(navi: Object, test: (x: number, y: number) -> boolean): (number?, number?)
    local require, forbid = panels.step_condition(navi)
    local found = {}
    for _, p in panels.filtered(require, forbid) do
        local x, y = unpacked(p)
        if test(x, y) then
            table.insert(found, p)
        end
    end
    return panels.pick(found)
end

-- Of the rows `test` passes, top to bottom, the panels the navi may step onto (`object_getPanelsInRowFiltered`, left
-- to right): one at random.
local function step_in_rows(navi: Object, test: (y: number) -> boolean): (number?, number?)
    local require, forbid = panels.step_condition(navi)
    local found = {}
    for y = 1, 3 do
        if test(y) then
            for x = 1, 6 do
                if field.check(x, y, require, forbid) then
                    table.insert(found, packed(x, y))
                end
            end
        end
    end
    return panels.pick(found)
end

-- ---- The classes --------------------------------------------------------------------------------------------

-- A class's routine: its steps by the step byte (the original's jump tables, `[step]` 0, 4, 8, 12), or one routine.
type Step = (navi: Object, s: State) -> ()

-- 0x080231CC (most classes' second step): no longer busy; in place if a hit would reach the target, else a step
-- anywhere of its own area.
local function arrived(navi: Object, s: State)
    s.place_busy = false
    if hittable(target_of(s)) then
        s.place_ready = true
        return
    end
    wander(navi)
end

-- 0x08023546, 0x08023B38, 0x08023DBA, 0x08024786 (a few classes' second step): in place.
local function done(_navi: Object, s: State)
    in_place(s)
end

-- 0x08023168 (`into_reach`, class 0): the chip reaches the target from where the navi stands: in place if a hit would
-- land, else a step anywhere of its own area; else a step to a panel from which it reaches (0x0800F910), or, with
-- none, the next target and a step anywhere.
local function into_reach(navi: Object, s: State)
    local t, range = target_of(s), range_of(chip_of(s))
    if reaches(navi, t, range) then
        if hittable(t) then
            return in_place(s)
        end
        return settle(navi, s)
    end
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, reach(range), navi.alliance)
    go_or(navi, s, x, y, give_up)
end

-- 0x0800F780 from the target: a panel on its row toward the navi's side that the navi may step onto, past `skip`
-- bodies of the navi's side or neutral ones and before the next; and where the last such body met stands.
local function behind(navi: Object, s: State, skip: number): (number?, number?, number)
    local t = target_of(s)
    local require, forbid = panels.step_condition(navi)
    local x, y, lx = panels.behind(navi, t.panel_x, t.panel_y, require, forbid, skip)
    return x, y, lx
end

-- 0x08023210 (`into_target_row`, class 1): a step to a panel of the target's row with nothing between; else the next
-- target and a step anywhere.
local function into_target_row(navi: Object, s: State)
    local x, y = behind(navi, s, 0)
    go_or(navi, s, x, y, give_up)
end

-- 0x08023264 (`behind_target`, class 2): the same; else past one body, when that one stands right behind the target;
-- else the next target and a step anywhere.
local function behind_target(navi: Object, s: State)
    local x, y = behind(navi, s, 0)
    if x ~= nil then
        return go(navi, s, x, y :: number)
    end
    local bx, by, lx = behind(navi, s, 1)
    if bx ~= nil and lx + panels.enemy_direction(navi) == target_of(s).panel_x then
        return go(navi, s, bx, by :: number)
    end
    give_up(navi, s)
end

-- 0x080232E0 (`past_obstacle`, class 3): a step to a panel of the target's row past one body between; else to one
-- with nothing between; else the next target and a step anywhere.
local function past_obstacle(navi: Object, s: State)
    local x, y = behind(navi, s, 1)
    if x == nil then
        x, y = behind(navi, s, 0)
    end
    go_or(navi, s, x, y, give_up)
end

-- 0x08023348 (`to_front`, class 4): the navi stands at its area's front (the panel toward its enemies isn't its
-- side's): in place if a hit would land, else a step anywhere; else a step to its area's front line (0x0800F990), or
-- the next target and a step anywhere.
local function to_front(navi: Object, s: State)
    local x, y = navi.panel_x + panels.enemy_direction(navi), navi.panel_y
    local p = field.panel(x, y)
    if field.valid(x, y) and p ~= nil and p.alliance ~= navi.alliance then
        if hittable(target_of(s)) then
            return in_place(s)
        end
        return settle(navi, s)
    end
    local fx, fy = panels.front_line(navi, navi.alliance, 0)
    go_or(navi, s, fx, fy, give_up)
end

-- 0x080233AC (`at_once`, class 5): in place at once.
local function at_once(_navi: Object, s: State)
    in_place(s)
end

-- 0x080233B8 (`when_counterable`, class 6): the target's counter window is open (its collision record's +0x0B): ready
-- if a hit would land, else not (no step); closed, the next target and a step anywhere.
local function when_counterable(navi: Object, s: State)
    s.place_busy = false
    local t = target_of(s)
    if t.collision.counter_timer ~= 0 then
        if hittable(t) then
            s.place_ready = true
        end
        return
    end
    place.skip_target(s)
    wander(navi)
end

-- A class that is a test: no longer busy; ready when it holds, else not and a step anywhere of its own area.
local function when(test: (navi: Object, s: State) -> boolean): Step
    return function(navi: Object, s: State)
        s.place_busy = false
        if test(navi, s) then
            s.place_ready = true
        else
            s.place_ready = false
            wander(navi)
        end
    end
end

-- 0x080233F0 (`when_hp_low`, class 7): the navi's HP is a tenth of its maximum or less. (Not ready, the ready flag
-- stays as the start left it: cleared.)
local function hp_low(navi: Object): boolean
    return navi.hp <= navi.max_hp // 10
end

-- 0x08023468: from the target on toward its own enemies, a panel no body stands on (0x10 clear) before the field's
-- end.
local function gap_before(t: Object): boolean
    local dir = panels.side_direction(t.alliance)
    local x, y = t.panel_x + dir, t.panel_y
    while field.valid(x, y) do
        if bit32.band(field.flags(x, y), STANDABLE) == 0 then
            return true
        end
        x += dir
    end
    return false
end

-- 0x0802349A: on that way, a body of the target's side or a neutral one.
local function body_before(t: Object): boolean
    local dir = panels.side_direction(t.alliance)
    local blocking = by_side(OWN_BODIES, t.alliance)
    local x, y = t.panel_x + dir, t.panel_y
    while field.valid(x, y) do
        if bit32.band(field.flags(x, y), blocking) ~= 0 then
            return true
        end
        x += dir
    end
    return false
end

-- 0x0802341C (`when_run_clear`, class 8, the MokoRus's): from the target to the far edge on the navi's side its row
-- has no gap and no body of its own: ready if a hit would land, else not (no step); else the next target and a step
-- anywhere.
local function when_run_clear(navi: Object, s: State)
    s.place_busy = false
    local t = target_of(s)
    if gap_before(t) or body_before(t) then
        place.skip_target(s)
        return wander(navi)
    end
    if hittable(t) then
        s.place_ready = true
    end
end

-- A panel the navi's chip lands right on: one a body stands on (0x10) that isn't lava.
local function landing(x: number, y: number): boolean
    return bit32.band(field.flags(x, y), STANDABLE) ~= 0 and kind_at(x, y) ~= "lava"
end

-- 0x080234F8 (`landing_ahead`, class 9): the panel toward its enemies is a landing: in place; else a step to a panel
-- whose panel ahead is one (0x08023550), or anywhere of its own area (still busy: the next decision searches again).
local function landing_ahead(navi: Object, s: State)
    local dir = panels.enemy_direction(navi)
    if landing(navi.panel_x + dir, navi.panel_y) then
        return in_place(s)
    end
    local x, y = step_where(navi, function(x: number, y: number): boolean
        return field.valid(x + dir, y) and landing(x + dir, y)
    end)
    if x ~= nil then
        return go(navi, s, x, y :: number)
    end
    wander(navi)
end

-- 0x080235FE's tests (a chip's record byte 23, less 32, the table at 0x08023624), of the target from the navi: 32, it
-- stands ahead and within a row of the navi's; 33, level or ahead and a row off; 34, on the edge of the other side's
-- three columns at the start (its top or bottom row, its back column or its front one).
local SPOTS: { [string]: (navi: Object, t: Object) -> boolean } = {
    ahead_within_a_row = function(navi: Object, t: Object): boolean
        return (t.panel_x - navi.panel_x) * panels.enemy_direction(navi) > 0 and math.abs(t.panel_y - navi.panel_y) <= 1
    end,
    a_row_off = function(navi: Object, t: Object): boolean
        return (t.panel_x - navi.panel_x) * panels.enemy_direction(navi) >= 0 and math.abs(t.panel_y - navi.panel_y) == 1
    end,
    on_far_ring = function(navi: Object, t: Object): boolean
        local side = navi.alliance
        return t.panel_y == 1 or t.panel_y == 3 or t.panel_x == 6 - 5 * side or t.panel_x == 4 - side
    end,
}

-- 0x080235FE: the spot test `name`.
local function spot_test(navi: Object, t: Object, name: string): boolean
    local test = SPOTS[name]
    if test == nil then
        -- (A reach list's number, under 32: 0x0802361E branches to itself.)
        error(string.format("the auto-battling navi's chip names %s where a spot test is read, and it is none", name))
    end
    return test(navi, t)
end

-- 0x080235C8 (`spot_test`, class 10): no step: the target stands where the chip's test says it reaches: in place if a
-- hit would land; else not; with the test failing, the next target too.
local function spot(navi: Object, s: State)
    local t = target_of(s)
    if spot_test(navi, t, range_of(chip_of(s))) then
        if hittable(t) then
            return in_place(s)
        end
        return not_ready(s)
    end
    place.skip_target(s)
    not_ready(s)
end

-- 0x080236AC (`three_ahead_near_enemy`, class 11): a panel it may step onto whose panel three ahead is on the field
-- and not one a body stands on, with an enemy on one of the eight panels around that one (the offset list `around`,
-- 16; the count's own pick draws): a step there; else not ready and a step anywhere.
local function three_ahead_near_enemy(navi: Object, s: State)
    local ahead = panels.enemy_direction(navi) * 3
    local enemies = by_side(ENEMY_BODIES, navi.alliance)
    local x, y = step_where(navi, function(x: number, y: number): boolean
        local ax = x + ahead
        if not field.valid(ax, y) or bit32.band(field.flags(ax, y), STANDABLE) ~= 0 then
            return false
        end
        local _, _, n = panels.random_relative(ax, y, enemies, 0, data.offsets.around, navi.alliance)
        return n ~= 0
    end)
    go_or(navi, s, x, y, settle)
end

-- 0x080238E0: the navis a search of the other side's reads, from the local console's list (lib/navi_list, BattleState
-- +0x44): from the other side's place in it, three entries when that side is the console's and one when it isn't. On
-- the navi's own console that is the other side's navi; on the other console it, then the navi itself, then none.
local function other_navis(navi: Object): { Object }
    local at = bit32.bxor(bit32.bxor(navi.alliance, 1), battle.local_side())
    local out = {}
    for i = at, if at == 0 then 2 else at do
        local o = navi_list.at(i)
        if o ~= nil then
            table.insert(out, o)
        end
    end
    return out
end

-- 0x0802385C: of those navis of element `element` (any: nil), the panels three before each (toward the navi's side)
-- that the navi may step onto: one at random.
local function before_enemies(navi: Object, element: number?): (number?, number?)
    local back = -panels.enemy_direction(navi) * 3
    local require, forbid = panels.step_condition(navi)
    local found = {}
    for _, e in other_navis(navi) do
        if element == nil or e.element == element then
            local x, y = e.panel_x + back, e.panel_y
            if field.check(x, y, require, forbid) then
                table.insert(found, packed(x, y))
            end
        end
    end
    return panels.pick(found)
end

-- The fire element's number (0x080237A2).
local FIRE = 1

-- 0x08023790 (`lava_ahead`, class 12): a panel it may step onto whose panel three ahead is lava (0x080237DA); else
-- three before a fire enemy, else before any; else not ready and a step anywhere.
local function lava_ahead(navi: Object, s: State)
    local ahead = panels.enemy_direction(navi) * 3
    local x, y = step_where(navi, function(x: number, y: number): boolean
        return field.valid(x + ahead, y) and kind_at(x + ahead, y) == "lava"
    end)
    if x == nil then
        x, y = before_enemies(navi, FIRE)
    end
    if x == nil then
        x, y = before_enemies(navi, nil)
    end
    go_or(navi, s, x, y, settle)
end

-- 0x08023918 (`gauge_into_reach`, class 13): with the custom gauge's damage (0x0801955A: formula 45, whose full
-- gauge gives 0 where the engine's gives 10, under 100 either way) 100 or more, a step to a panel from which the
-- reach list `two_columns_ahead` (11) reaches the target; else not ready and a step anywhere.
local function gauge_into_reach(navi: Object, s: State)
    if battle.gauge_damage(navi.alliance) >= 100 then
        local t = target_of(s)
        local require, forbid = panels.step_condition(navi)
        local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, data.ranges.two_columns_ahead, navi.alliance)
        if x ~= nil then
            return go(navi, s, x, y :: number)
        end
    end
    settle(navi, s)
end

-- A `sword_swing` chip's swings by the side's pick (its `auto_battle.swings`: 0x08023A68, VarSwrd's six from byte 0,
-- NeoVari's four from byte 6, which 0x08023990 and its kin pick by the chip, 0xD9): each a reach list, or a spot
-- test.
local function swing_of(navi: Object, s: State): string
    local swings = record(chip_of(s)).swings
    if swings == nil then
        error(string.format("%s's positioning class sword_swing reads its swings, and it names none", chip_of(s).id :: string))
    end
    local swing: string? = swings[battle.sword_pick(navi.alliance) + 1]
    if swing == nil then
        error(string.format("the side's sword pick %d is past its chip's swings (0x08023A68)", battle.sword_pick(navi.alliance)))
    end
    return swing
end

-- 0x08023980 (`sword_swing`, class 14, VarSwrd's and NeoVari's): the side's sword pick (the side's +0x12, which the
-- sword reads) drawn at random among the chip's swings; the swing a spot test (VarSwrd's pick 4, NeoVari's 1, by
-- number: 0x080239A8), the third step next, else the second.
local function swing_pick(navi: Object, s: State)
    local swings = record(chip_of(s)).swings or {}
    battle.set_sword_pick(navi.alliance, battle.rng_positive() % #swings)
    s.place_step = if SPOTS[swing_of(navi, s)] ~= nil then 8 else 4
end

-- 0x080239B2 (`sword_swing`'s second step): the swing reaches the target from where the navi stands: in place if a
-- hit would land, else a step anywhere; else a step to a panel from which it reaches (the fourth step next), or the
-- next target and a step anywhere.
local function swing_reach(navi: Object, s: State)
    local t, range = target_of(s), swing_of(navi, s)
    if reaches(navi, t, range) then
        if hittable(t) then
            return in_place(s)
        end
        return settle(navi, s)
    end
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, reach(range), navi.alliance)
    if x == nil then
        return give_up(navi, s)
    end
    s.place_step = 12
    move(navi, x, y :: number)
end

-- 0x08023A22 (`sword_swing`'s third step): the swing's spot test: the target stands there: in place if a hit would
-- land, else nothing (still busy: the next decision tests again); else not ready and the next target.
local function swing_spot(navi: Object, s: State)
    local t = target_of(s)
    if spot_test(navi, t, swing_of(navi, s)) then
        if hittable(t) then
            in_place(s)
        end
        return
    end
    place.skip_target(s)
    not_ready(s)
end

-- Panel conditions by side: an enemy in the side's area (0x08023AE4); a panel of the side's area no body stands on
-- (0x08023C94); the side's area (0x08023EEC); an enemy in its own area (0x08024648), and on a cracked panel or on
-- grass there (0x08024498, 0x08024510).
local ENEMY_IN_AREA: { { number } } = { { 0x04000000, 0x20 }, { 0x08000020, 0 } }
local HOLE_IN_AREA: { { number } } = { { 0, 0x30 }, { 0x20, 0x10 } }
local OWN_AREA: { { number } } = { { 0, 0x20 }, { 0x20, 0 } }
local ENEMY_AT_HOME: { { number } } = { { 0x04000020, 0 }, { 0x08000000, 0x20 } }
local ENEMY_ON_CRACKED: { { number } } = { { 0x04000060, 0 }, { 0x08000040, 0x20 } }
local ENEMY_ON_GRASS: { { number } } = { { 0x04000420, 0 }, { 0x08000400, 0x20 } }

-- 0x08023A90 (`when_enemy_in_area`, class 15): an enemy stands in the navi's area.
local function enemy_in_area(navi: Object): boolean
    return count_panels(ENEMY_IN_AREA, navi.alliance) ~= 0
end

-- 0x08023B10 (`no_hole_ahead`, class 16): a step to a panel whose panel toward its enemies is on the field and no
-- hole (0x08023B42), or anywhere of its own area (still busy: the next decision searches again).
local function no_hole_ahead(navi: Object, s: State)
    local dir = panels.enemy_direction(navi)
    local x, y = step_where(navi, function(x: number, y: number): boolean
        return field.valid(x + dir, y) and kind_at(x + dir, y) ~= "hole"
    end)
    if x ~= nil then
        return go(navi, s, x, y :: number)
    end
    wander(navi)
end

-- 0x08023BD0 (`when_front_free`, class 17): one of its area's front panels is free to step onto (0x08023BFA: the
-- first two rows of the step conditions, whatever its shoes).
local FRONT_FREE: { { number } } = { { 0x10, 0x0F8800A0 }, { 0x30, 0x0F880080 } }
local function front_free(navi: Object): boolean
    local c = by_side(FRONT_FREE, navi.alliance)
    return #panels.area_fronts(navi.alliance, 0, c[1], c[2]) ~= 0
end

-- 0x08023C48 (`when_hole_in_area`, class 18): a panel of the navi's area no body stands on.
local function hole_in_area(navi: Object): boolean
    return count_panels(HOLE_IN_AREA, navi.alliance) ~= 0
end

-- 0x080E4D94 (TimeBomb's own search): the first column, from the navi's side's edge on, with a panel meeting the
-- conditions; none: nil.
local function first_column(side: number, require: number, forbid: number): number?
    local dir = panels.side_direction(side)
    local x = if side == 0 then 1 else 6
    while x >= 1 and x <= 6 do
        for y = 1, 3 do
            if field.check(x, y, require, forbid) then
                return x
            end
        end
        x += dir
    end
    return nil
end

-- 0x08023CC0 (`when_far_front_free`, class 19, TimeBomb's): the first column with a panel of the other side's that a
-- body stands on (0x08023D50, 0x08023D58) has one free of bodies (0x08023D60, 0x08023D68).
local FAR_FRONT: { { number } } = { { 0x30, 0 }, { 0x10, 0x20 } }
local FAR_FRONT_FREE: { { number } } = { { 0x30, 0x0F880080 }, { 0x10, 0x0F8800A0 } }
local function far_front_free(navi: Object): boolean
    local c = by_side(FAR_FRONT, navi.alliance)
    local x = first_column(navi.alliance, c[1], c[2])
    if x == nil then
        return false
    end
    local free = by_side(FAR_FRONT_FREE, navi.alliance)
    for y = 3, 1, -1 do
        if field.check(x, y, free[1], free[2]) then
            return true
        end
    end
    return false
end

-- 0x08023D8C (`three_ahead_open`, class 20): a panel it may step onto whose panel three ahead is on the field, one a
-- body stands on and free of bodies and the side flags (0x0F880080 clear): a step there; else not ready and a step
-- anywhere.
local function three_ahead_open(navi: Object, s: State)
    local ahead = panels.enemy_direction(navi) * 3
    local x, y = step_where(navi, function(x: number, y: number): boolean
        local flags = field.flags(x + ahead, y)
        return field.valid(x + ahead, y) and bit32.band(flags, STANDABLE) ~= 0 and bit32.band(flags, 0x0F880080) == 0
    end)
    go_or(navi, s, x, y, settle)
end

-- 0x08023E44 (`when_object_hurt`, class 21, Repair's): one of the side's field objects (the registry's three slots,
-- BattleState +0x68) has lost HP.
local function object_hurt(navi: Object): boolean
    for i = 1, 3 do
        local o = field.object_slot(navi.alliance, i)
        if o ~= nil and o.hp < o.max_hp then
            return true
        end
    end
    return false
end

-- 0x08023EA0 (`when_area_lost`, class 22): the navi's area is 8 panels or fewer. 0x08023F5C (`when_area_whole`, class
-- 24): 9 or more.
local function area_lost(navi: Object): boolean
    return count_panels(OWN_AREA, navi.alliance) <= 8
end
local function area_whole(navi: Object): boolean
    return count_panels(OWN_AREA, navi.alliance) >= 9
end

-- 0x08023EFC (`when_panel_changed`, class 23): a panel the side holds is neither normal nor holy (0x08023F28).
local function panel_changed(navi: Object): boolean
    for y = 1, 3 do
        for x = 1, 6 do
            local p = field.panel(x, y)
            if p ~= nil and p.alliance == navi.alliance and p.kind ~= "holy" and p.kind ~= "normal" then
                return true
            end
        end
    end
    return false
end

-- 0x08023F8C (`when_visible`, class 25): the navi is neither invisible nor submerged (its flags 6).
local function visible(navi: Object): boolean
    return not (navi:status("invisible") or navi:status("submerged"))
end

-- 0x08023FBC (`when_unbarriered`, class 26): the navi has no barrier (its collision record's +0x04).
local function unbarriered(navi: Object): boolean
    return navi.collision.barrier == "none"
end

-- 0x08023FE8 (`when_enemy_barrier`, class 27): the first of the other side's navis (0x080238E0's) has a barrier. (The
-- loop means each of them, but it steps the record it read, not the list: 0x0802402A.)
local function enemy_barrier(navi: Object): boolean
    local e = other_navis(navi)[1]
    return e ~= nil and e.collision.barrier ~= "none"
end

-- 0x08024054 (`own_panel_ahead`, class 28): a step to a panel whose panel toward its enemies is the navi's side's and
-- not holy (0x08024080); else not ready and a step anywhere.
local function own_panel_ahead(navi: Object, s: State)
    local dir = panels.enemy_direction(navi)
    local x, y = step_where(navi, function(x: number, y: number): boolean
        local ahead = field.panel(x + dir, y)
        return field.valid(x + dir, y) and ahead ~= nil and ahead.alliance == navi.alliance and ahead.kind ~= "holy"
    end)
    go_or(navi, s, x, y, settle)
end

-- 0x080240F8 (`when_trap_free`, class 29, the traps): the navi's side holds no defensive chip (0x08022FCC), or an
-- element trap (chips 0x8C to 0x8F: their `auto_battle.element_trap`), which they may set over one another.
local function trap_free(navi: Object): boolean
    local held = battle.linked(navi.alliance).chip
    if held == nil then
        return true
    end
    local r = (held :: any).auto_battle :: AutoBattle?
    return r ~= nil and r.element_trap == true
end

-- 0x08024154 (`into_empty_row`, class 30): a step into a row none of whose panels has a body or a side flag
-- (0x08024182: 0x0F880080); else not ready and a step anywhere.
local function into_empty_row(navi: Object, s: State)
    local x, y = step_in_rows(navi, function(y: number): boolean
        for x = 1, 6 do
            if bit32.band(field.flags(x, y), 0x0F880080) ~= 0 then
                return false
            end
        end
        return true
    end)
    go_or(navi, s, x, y, settle)
end

-- 0x080241E4 (`when_any_obstacle`, class 31): a body that is no navi's stands somewhere (0x08024210: the panels'
-- flags 0x03800000).
local function any_obstacle(_navi: Object): boolean
    for y = 1, 3 do
        for x = 1, 6 do
            if bit32.band(field.flags(x, y), 0x03800000) ~= 0 then
                return true
            end
        end
    end
    return false
end

-- 0x0800A1AE: row `y`'s first panel meeting the conditions, from side `side`'s far edge back toward its own; none:
-- nil.
local function first_in_row(side: number, y: number, require: number, forbid: number): number?
    local back = -panels.side_direction(side)
    local x = 6 - 5 * side
    while field.valid(x, y) do
        if field.check(x, y, require, forbid) then
            return x
        end
        x += back
    end
    return nil
end

-- 0x0802425C (`into_open_row`, class 32): a step into a row whose first panel of the other side's, from the navi's
-- side on, is one a body stands on and free of bodies (0x0802428A); else not ready and a step anywhere.
local OTHERS_PANEL: { { number } } = { { 0x20, 0 }, { 0, 0x20 } }
local function into_open_row(navi: Object, s: State)
    local c = by_side(OTHERS_PANEL, navi.alliance)
    local x, y = step_in_rows(navi, function(y: number): boolean
        local fx = first_in_row(bit32.bxor(navi.alliance, 1), y, c[1], c[2])
        return fx ~= nil and field.check(fx, y, STANDABLE, ANY_BODY)
    end)
    go_or(navi, s, x, y, settle)
end

-- 0x080243E8: from (x, y) on toward side `side`'s enemies, a panel one of them stands on before the field's end.
local function enemy_ahead(x: number, y: number, side: number): boolean
    local dir = panels.side_direction(side)
    local enemies = by_side(ENEMY_BODIES, side)
    x += dir
    while field.valid(x, y) do
        if bit32.band(field.flags(x, y), enemies) ~= 0 then
            return true
        end
        x += dir
    end
    return false
end

-- 0x0802432C (`own_panel_before_enemy`, class 33): a step to a panel whose panel toward its enemies is the navi's
-- side's, one a body stands on and free of bodies, with an enemy further along the row (0x0802435A); else not ready
-- and a step anywhere.
local function own_panel_before_enemy(navi: Object, s: State)
    local dir = panels.enemy_direction(navi)
    local x, y = step_where(navi, function(x: number, y: number): boolean
        local ax = x + dir
        local ahead = field.panel(ax, y)
        return field.valid(ax, y)
            and ahead ~= nil
            and ahead.alliance == navi.alliance
            and field.check(ax, y, STANDABLE, ANY_BODY)
            and enemy_ahead(ax, y, navi.alliance)
    end)
    go_or(navi, s, x, y, settle)
end

-- A class that waits where the navi stands: with a panel meeting the side's conditions the second step next (no
-- step, still busy); else not ready and a step anywhere.
local function wait_for(conds: { { number } }): Step
    return function(navi: Object, s: State)
        if count_panels(conds, navi.alliance) ~= 0 then
            s.place_step = 4
            return
        end
        settle(navi, s)
    end
end

-- The story's navis' flag the hurt test skips for (NaviStats +0x26 2: 0x08024544), which no netbattle's navi has.
-- 0x0802453C (`hurt_into_reach`, class 36, Muramasa's): with less than 300 HP lost, not ready (no step); else as
-- `into_reach`, but in reach the second step comes without a look at the target.
local function hurt_into_reach(navi: Object, s: State)
    if navi.max_hp - navi.hp < 300 then
        return not_ready(s)
    end
    local t, range = target_of(s), range_of(chip_of(s))
    if reaches(navi, t, range) then
        s.place_step = 4
        return
    end
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, reach(range), navi.alliance)
    go_or(navi, s, x, y, give_up)
end

-- 0x080245B8 (`when_bugged`, class 37): side 0's navi has a NaviCust bug (0x08043014, rules/navicust's `bug_kinds`):
-- side 0's whoever asks, the routine handed the 0 just written to the busy flag where it takes a side.
local function bugged(_navi: Object): boolean
    return navicust.bug_kinds(0) ~= 0
end

-- 0x08024674 (`two_solid_ahead`, class 39): a step to a panel whose panel toward its enemies is one a body stands on,
-- free of bodies, and the one after it one a body stands on (0x080246A2); else not ready and a step anywhere.
local function two_solid_ahead(navi: Object, s: State)
    local dir = panels.enemy_direction(navi)
    local x, y = step_where(navi, function(x: number, y: number): boolean
        return field.valid(x + dir, y)
            and field.check(x + dir, y, STANDABLE, ANY_BODY)
            and field.valid(x + 2 * dir, y)
            and bit32.band(field.flags(x + 2 * dir, y), STANDABLE) ~= 0
    end)
    go_or(navi, s, x, y, settle)
end

-- 0x08024830, 0x08024850: row `y` has a hole; has a panel that is none.
local function row_has(y: number, hole: boolean): boolean
    for x = 1, 6 do
        if (kind_at(x, y) == "hole") == hole then
            return true
        end
    end
    return false
end

-- 0x0802474C (`into_whole_row`, class 40): a step into a row with no hole (0x08024790); else into one that isn't all
-- holes (0x080247E0); else not ready and a step anywhere.
local function into_whole_row(navi: Object, s: State)
    local x, y = step_in_rows(navi, function(y: number): boolean
        return not row_has(y, true)
    end)
    if x == nil then
        x, y = step_in_rows(navi, function(y: number): boolean
            return row_has(y, false)
        end)
    end
    go_or(navi, s, x, y, settle)
end

-- 0x0802488C (`to_front_before_solid`, class 41): a step to one of its area's front panels whose panel toward its
-- enemies is one a body stands on (0x080248BE); else not ready, the next target and a step anywhere.
local function to_front_before_solid(navi: Object, s: State)
    local dir = panels.enemy_direction(navi)
    local require, forbid = panels.step_condition(navi)
    local found = {}
    for _, p in panels.area_fronts(navi.alliance, 0, require, forbid) do
        local x, y = unpacked(p)
        if bit32.band(field.flags(x + dir, y), STANDABLE) ~= 0 then
            table.insert(found, p)
        end
    end
    local x, y = panels.pick(found)
    go_or(navi, s, x, y, give_up)
end

-- 0x08024938 (`when_enemy_approachable`, class 42): the panel before an enemy, on the navi's side of it, is one a
-- body stands on and free of bodies (0x08024962): the second step next (no step); else not ready and a step anywhere.
local function when_enemy_approachable(navi: Object, s: State)
    local back = -panels.enemy_direction(navi)
    for _, p in panels.filtered(by_side(ENEMY_BODIES, navi.alliance), 0) do
        local x, y = unpacked(p)
        if field.check(x + back, y, STANDABLE, ANY_BODY) then
            s.place_step = 4
            return
        end
    end
    settle(navi, s)
end

-- The eight panels around one (0x08024A98), as the search lists them.
local AROUND: { { number } } = { { 1, -1 }, { 1, 0 }, { 1, 1 }, { 0, 1 }, { -1, 1 }, { -1, 0 }, { -1, -1 }, { 0, -1 } }

-- 0x080249DC (`three_before_around_target`, class 43): of the eight panels around the target that a body stands on
-- and none does, the panels three before each (toward the navi's side) that the navi may step onto (0x08024A18): a
-- step to one; else not ready, the next target and a step anywhere.
local function three_before_around_target(navi: Object, s: State)
    local t = target_of(s)
    local back = -panels.enemy_direction(navi) * 3
    local require, forbid = panels.step_condition(navi)
    local found = {}
    for _, d in AROUND do
        local x, y = t.panel_x + d[1], t.panel_y + d[2]
        if field.check(x, y, STANDABLE, ANY_BODY) and field.check(x + back, y, require, forbid) then
            table.insert(found, packed(x + back, y))
        end
    end
    local x, y = panels.pick(found)
    go_or(navi, s, x, y, give_up)
end

-- 0x08024ACC (`three_before_target`, class 44): a step to the panel three before the target, if the navi may step
-- there, a body stands on it and none does, and it may step onto the panel two before too (0x08024B08); else not
-- ready, the next target and a step anywhere.
local function three_before_target(navi: Object, s: State)
    local t = target_of(s)
    local back = -panels.enemy_direction(navi)
    local require, forbid = panels.step_condition(navi)
    local x, y = t.panel_x + 3 * back, t.panel_y
    if
        field.check(x, y, require, forbid)
        and field.check(x, y, STANDABLE, ANY_BODY)
        and field.check(t.panel_x + 2 * back, y, require, forbid)
    then
        return go(navi, s, x, y)
    end
    give_up(navi, s)
end

-- The bodies that end `into_target_row_on_ground`'s walk (0x08024C74: a side's own, the other side's navis' and
-- neutral ones).
local GROUND_BLOCKING: { number } = { 0x05800000, 0x0A800000 }

-- 0x08024B7C (`into_target_row_on_ground`, class 45): a step to a panel of the target's row, toward the navi's side,
-- with no body and no panel that none stands on between (0x08024BB4); else the next target and a step anywhere.
local function into_target_row_on_ground(navi: Object, s: State)
    local t = target_of(s)
    local back = -panels.side_direction(navi.alliance)
    local blocking = by_side(GROUND_BLOCKING, navi.alliance)
    local require, forbid = panels.step_condition(navi)
    local found = {}
    local x, y = t.panel_x + back, t.panel_y
    while true do
        local flags = field.flags(x, y)
        if bit32.band(flags, blocking) ~= 0 or bit32.band(flags, STANDABLE) == 0 then
            break
        end
        if field.check(x, y, require, forbid) then
            table.insert(found, packed(x, y))
        end
        x += back
        if not field.valid(x, y) then
            break
        end
    end
    local px, py = panels.pick(found)
    go_or(navi, s, px, py, give_up)
end

-- A two-step class: its first step, then `arrived`.
local function steps(first: Step): { [number]: Step }
    return { [0] = first, [4] = arrived }
end

-- The classes (0x0800F3C8's routines, by the name each chip's `auto_battle` gives its own): their steps by the step
-- byte, or one routine.
local CLASSES: { [string]: { [number]: Step } | Step } = {
    into_reach = steps(into_reach),
    into_target_row = steps(into_target_row),
    behind_target = steps(behind_target),
    past_obstacle = steps(past_obstacle),
    to_front = steps(to_front),
    at_once = at_once,
    when_counterable = when_counterable,
    when_hp_low = when(hp_low),
    when_run_clear = when_run_clear,
    landing_ahead = { [0] = landing_ahead, [4] = done },
    spot_test = spot,
    three_ahead_near_enemy = steps(three_ahead_near_enemy),
    lava_ahead = steps(lava_ahead),
    gauge_into_reach = steps(gauge_into_reach),
    sword_swing = { [0] = swing_pick, [4] = swing_reach, [8] = swing_spot, [12] = arrived },
    when_enemy_in_area = when(enemy_in_area),
    no_hole_ahead = { [0] = no_hole_ahead, [4] = done },
    when_front_free = when(front_free),
    when_hole_in_area = when(hole_in_area),
    when_far_front_free = when(far_front_free),
    three_ahead_open = { [0] = three_ahead_open, [4] = done },
    when_object_hurt = when(object_hurt),
    when_area_lost = when(area_lost),
    when_panel_changed = when(panel_changed),
    when_area_whole = when(area_whole),
    when_visible = when(visible),
    when_unbarriered = when(unbarriered),
    when_enemy_barrier = when(enemy_barrier),
    own_panel_ahead = steps(own_panel_ahead),
    when_trap_free = when(trap_free),
    into_empty_row = steps(into_empty_row),
    when_any_obstacle = when(any_obstacle),
    into_open_row = steps(into_open_row),
    own_panel_before_enemy = steps(own_panel_before_enemy),
    when_enemy_on_cracked = steps(wait_for(ENEMY_ON_CRACKED)),
    when_enemy_on_grass = steps(wait_for(ENEMY_ON_GRASS)),
    hurt_into_reach = steps(hurt_into_reach),
    when_bugged = when(bugged),
    when_enemy_at_home = steps(wait_for(ENEMY_AT_HOME)),
    two_solid_ahead = steps(two_solid_ahead),
    into_whole_row = { [0] = into_whole_row, [4] = done },
    to_front_before_solid = steps(to_front_before_solid),
    when_enemy_approachable = steps(when_enemy_approachable),
    three_before_around_target = steps(three_before_around_target),
    three_before_target = steps(three_before_target),
    into_target_row_on_ground = steps(into_target_row_on_ground),
}

-- 0x0800F394: one decision's step: busy until the class's routine says otherwise; whether it still is.
function place.step(navi: Object, s: State): boolean
    s.place_busy = true
    local class = record(chip_of(s)).class
    local c = CLASSES[class]
    if c == nil then
        error(string.format("the auto-battling navi's chip has positioning class %s, which the AI can't play (past the table at 0x0800F3C8)", class))
    end
    if type(c) == "function" then
        (c :: Step)(navi, s)
    else
        local step = (c :: { [number]: Step })[s.place_step]
        if step == nil then
            error(string.format("positioning class %s's step %#x reads past its table", class, s.place_step))
        end
        step(navi, s)
    end
    return s.place_busy
end

return place
