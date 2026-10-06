--!strict
-- A navi in EXE5's auto battle getting in place for a chip (0x08029B3C, from the AI's chip play, ./ai): each chip's record
-- names how (byte 13, its positioning class: data.chips[...].class), and the class's routine runs once a decision
-- until it says it is no longer busy. Most classes have two steps: a search for a panel the chip reaches the target
-- from, a step there, and the next decision's "in place"; the rest test a condition and say at once. In place or
-- not, the AI plays the chip once the navi is no longer busy (it reads only `busy`; `ready` is the AI's other users'),
-- after one more step anywhere of its own area when the search found nothing. Its state is the navi's (AIData +0x130
-- on: the step +0x130, +0x131, busy +0x132, ready +0x133, the chip +0x134, the target +0x138).

local data = require("./data")
local panels = require("./panels")
local operation_battle = require("../../lib/operation_battle")

local place = {}

-- What a class's routine reads and writes of the AI's state (./ai's State: the place fields, and the chip's target
-- search's index).
type State = {
    place_step: number,
    place_step_init: number,
    place_busy: boolean,
    place_ready: boolean,
    place_chip: Chip?,
    place_target: Object?,
    next_target: number,
}

-- 0x08029BEC: getting in place for `chip` against `target` starts (the step 0; ready cleared; busy as it was).
function place.start(s: State, chip: Chip, target: Object)
    s.place_chip, s.place_target = chip, target
    s.place_step, s.place_step_init = 0, 0
    s.place_ready = false
end

-- 0x0802BE06: the chip's next target search starts one further on (./ai's targets, 0 to 4).
function place.skip_target(s: State)
    s.next_target += 1
    if s.next_target > 4 then
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

-- What the AI reads of the chip's record (data.chips).
local function record(chip: Chip)
    local r = data.chips[chip.id]
    if r == nil then
        error(string.format("the navi in auto battle plays %s, which the AI's data (./data) doesn't list", chip.id))
    end
    return r
end

-- The chip's reach list (record byte 23, data.ranges; `sub_8022AF4`).
local function reach(range: number): { { number } }
    local list = data.ranges[range]
    if list == nil then
        error(string.format("the auto-battling navi's chip names reach list %d, past the lists (sub_8022AF4)", range))
    end
    return list
end

-- 0x08029C16 (`dir` the navi's enemies' way) and 0x08029CB2 (the way it faces): one of the reach list's panels from
-- the navi (dx along `dir`) is the target's.
local function reaches(navi: Object, target: Object, range: number, dir: number): boolean
    for _, d in reach(range) do
        if navi.panel_x + d[1] * dir == target.panel_x and navi.panel_y + d[2] == target.panel_y then
            return true
        end
    end
    return false
end

-- 0x08029C00: a hit would reach the target: it is neither invisible, invulnerable nor flashing (its flags 0x20A).
local function hittable(target: Object): boolean
    return not (target:status("invisible") or target:status("invulnerable") or target:status("flashing"))
end

-- `sub_80116F6`: a step to (x, y), the step's end lag its own (`sub_8010332`), facing the target.
local function move(navi: Object, s: State, x: number, y: number)
    navi:start_move_to(x, y, navi:move_lag(), s.place_target)
end

-- 0x08029D7E (most classes' second step): in place.
local function in_place(s: State)
    s.place_busy, s.place_ready = false, true
end

local function not_ready(s: State)
    s.place_busy, s.place_ready = false, false
end

-- `sub_8015C48`: a step anywhere of its own area.
local function wander(navi: Object, s: State)
    local x, y = panels.random_own(navi)
    move(navi, s, x, y)
end

-- A step to (x, y), the class's step `step` next decision (the step's halfword: its byte after it cleared).
local function go(navi: Object, s: State, step: number, x: number, y: number)
    s.place_step, s.place_step_init = step, 0
    move(navi, s, x, y)
end

-- The search found nothing: not ready, a step anywhere of its own area.
local function settle(navi: Object, s: State)
    not_ready(s)
    wander(navi, s)
end

-- The same, and the chip's next target search starts one further on.
local function give_up(navi: Object, s: State)
    place.skip_target(s)
    settle(navi, s)
end

-- The panel (x, y)'s type.
local function kind_at(x: number, y: number): PanelType?
    local p = field.panel(x, y)
    return if p then p.kind else nil
end

-- `object_dead_getPanelsTypeAllianceCount`: the panels (rows 1 to 3, columns 1 to 6) of type `kind` that side
-- `side` holds.
local function count_type(kind: PanelType, side: number): number
    local n = 0
    for y = 1, 3 do
        for x = 1, 6 do
            local p = field.panel(x, y)
            if p and p.alliance == side and p.kind == kind then
                n += 1
            end
        end
    end
    return n
end

-- The panels' flag 0x10, which the step conditions require of a panel (`sub_8010368`'s table).
local STANDABLE = 0x10

-- A panel's position packed as the original lists it.
local function packed(x: number, y: number): number
    return bit32.bor(x, bit32.lshift(y, 4))
end

-- One of `list` (packed) at random (`GetPositiveSignedRNG2` modulo how many); none: nil and no draw.
local function pick(list: { number }): (number?, number?)
    if #list == 0 then
        return nil, nil
    end
    local p = list[battle.rng_positive() % #list + 1]
    return bit32.band(p, 7), bit32.rshift(p, 4)
end

-- ---- The classes --------------------------------------------------------------------------------------------

-- A class's routine: its steps by the step byte (the original's jump tables, `[step]` 0, 4, 8), or one routine.
type Step = (navi: Object, s: State) -> ()

-- 0x08029D18 (classes 0 and 17, and 7 in the operation battle): the chip reaches the target from where the navi stands
-- (its reach list toward its enemies): in place if a hit would land, else a step anywhere of its own area; else a
-- step to a panel from which it reaches, or, with none, the next target and a step anywhere.
local function reach_from_here(navi: Object, s: State)
    local t, range = target_of(s), record(chip_of(s)).range
    if reaches(navi, t, range, panels.enemy_direction(navi)) then
        if hittable(t) then
            return in_place(s)
        end
        return settle(navi, s)
    end
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, range, navi.alliance)
    if x == nil then
        return give_up(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- 0x08029DBC (class 1): the navi stands at its area's front (the panel it faces isn't its side's): in place if a
-- hit would land, else a step anywhere; else a step to its area's front line (EXE5's 0x08013126), or the next target
-- and a step anywhere.
local function front(navi: Object, s: State)
    local x, y = navi.panel_x + panels.front_direction(navi), navi.panel_y
    local p = field.panel(x, y)
    if field.valid(x, y) and p ~= nil and p.alliance ~= navi.alliance then
        if hittable(target_of(s)) then
            return in_place(s)
        end
        return settle(navi, s)
    end
    local fx, fy = panels.front_line(navi, navi.alliance, navi.flip)
    if fx == nil then
        return give_up(navi, s)
    end
    go(navi, s, 4, fx, fy :: number)
end

-- 0x08029E5A's two tests (record byte 23 less 32, the table at 0x08029E80): 32, the target stands in its area's
-- back column (as the navi faces: column 6 facing right, else 1) or the top or bottom row; 33, the panel behind
-- the target and a row up is one a body stands on and empty (0x10 set, 0x0F800000 clear).
local SPOTS: { [number]: (navi: Object, t: Object) -> boolean } = {
    [32] = function(navi: Object, t: Object): boolean
        local back = if panels.front_direction(navi) > 0 then 6 else 1
        return t.panel_x == back or t.panel_y == 1 or t.panel_y == 3
    end,
    [33] = function(navi: Object, t: Object): boolean
        return field.check(t.panel_x - panels.front_direction(navi), t.panel_y - 1, STANDABLE, 0x0F800000)
    end,
}

-- 0x08029E24 (class 2): no step: the target stands where the chip's test (above) says it reaches: in place if a hit
-- would land; else not; with the test failing, the next target too. The navi doesn't move.
local function spot(navi: Object, s: State)
    local range = record(chip_of(s)).range
    if range < 32 then
        -- 0x08029E7A branches to itself.
        error(string.format("the auto-battling navi's chip of positioning class 2 has reach %d: 0x08029E7A loops forever", range))
    end
    local test = SPOTS[range]
    if test == nil then
        error(string.format("the auto-battling navi's chip of positioning class 2 has reach %d, past the table at 0x08029E80", range))
    end
    local t = target_of(s)
    if test(navi, t) then
        if hittable(t) then
            return in_place(s)
        end
        return not_ready(s)
    end
    place.skip_target(s)
    not_ready(s)
end

-- A panel the navi's chip lands right on: one a body stands on (0x10) that isn't lava.
local function landing(x: number, y: number): boolean
    return bit32.band(field.flags(x, y), STANDABLE) ~= 0 and kind_at(x, y) ~= "lava"
end

-- 0x08029EE4 (class 3): the panel toward its enemies is a landing: in place; else a step to a panel it may stand on
-- whose panel ahead (as it faces) is one (0x08029F3E), or anywhere of its own area (still busy: the next decision
-- searches again).
local function landing_ahead(navi: Object, s: State)
    if landing(navi.panel_x + panels.enemy_direction(navi), navi.panel_y) then
        return in_place(s)
    end
    local require, forbid = panels.step_condition(navi)
    local dir = panels.front_direction(navi)
    local found = {}
    for _, p in panels.filtered(require, forbid) do
        local x, y = bit32.band(p, 7), bit32.rshift(p, 4)
        if field.valid(x + dir, y) and landing(x + dir, y) then
            table.insert(found, packed(x, y))
        end
    end
    local x, y = pick(found)
    if x ~= nil then
        return go(navi, s, 4, x, y :: number)
    end
    wander(navi, s)
end

-- 0x08029FD4 (class 4): a panel behind the target on its row it may step onto, before anything blocks it (`sub_8015CC0`
-- past none); else past one blocking panel, when that one stands right behind the target; else the next target and
-- a step anywhere.
local function behind(navi: Object, s: State)
    local t = target_of(s)
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.behind(navi, t.panel_x, t.panel_y, require, forbid, 0)
    if x ~= nil then
        return go(navi, s, 4, x, y :: number)
    end
    require, forbid = panels.step_condition(navi)
    local bx, by, lx = panels.behind(navi, t.panel_x, t.panel_y, require, forbid, 1)
    if bx ~= nil and lx + panels.front_direction(navi) == t.panel_x then
        return go(navi, s, 4, bx, by :: number)
    end
    give_up(navi, s)
end

-- The other side's bodies a side looks for (0x0802A114: panel flag 0x04000000 for side 0, 0x08000000 for 1).
local ENEMY_BODIES: { number } = { 0x04000000, 0x08000000 }

-- 0x0802A050 (class 5): a panel it may step onto whose panel three ahead (as it faces) is on the field and not one a
-- body stands on (0x10 clear), with an enemy on one of the eight panels around that one (offset list 16): a step
-- there; else not ready and a step anywhere.
local function three_ahead_near_enemy(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local ahead = panels.front_direction(navi) * 3
    local enemies = ENEMY_BODIES[bit32.band(navi.alliance, 1) + 1]
    local found = {}
    for _, p in panels.filtered(require, forbid) do
        local x, y = bit32.band(p, 7), bit32.rshift(p, 4)
        local ax = x + ahead
        if field.valid(ax, y) and bit32.band(field.flags(ax, y), STANDABLE) == 0 then
            local _, _, n = panels.random_offset(ax, y, enemies, 0, 16, navi.alliance)
            if n ~= 0 then
                table.insert(found, packed(x, y))
            end
        end
    end
    local x, y = pick(found)
    if x == nil then
        return settle(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- 0x0802A208: the other side's first four alive actors (BattleState +0x80) of element `element` (any: 0xFF): the
-- panels three before each (toward the navi, as it faces) that meet the conditions, one at random.
local function before_enemies(navi: Object, element: number, require: number, forbid: number): (number?, number?)
    local back = -panels.front_direction(navi) * 3
    local other = bit32.bxor(navi.alliance, 1)
    local found = {}
    for i = 1, 4 do
        local e = battle.alive_actor_slot(other, i)
        if e ~= nil and (element == 0xFF or e.element == element) then
            local x, y = e.panel_x + back, e.panel_y
            if field.check(x, y, require, forbid) then
                table.insert(found, packed(x, y))
            end
        end
    end
    return pick(found)
end

-- 0x0802A138 (class 6): a panel it may step onto whose panel three ahead is lava (0x0802A184); else three before a
-- fire enemy, else before any; else not ready and a step anywhere.
local function lava_ahead(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local ahead = panels.front_direction(navi) * 3
    local found = {}
    for _, p in panels.filtered(require, forbid) do
        local x, y = bit32.band(p, 7), bit32.rshift(p, 4)
        if field.valid(x + ahead, y) and kind_at(x + ahead, y) == "lava" then
            table.insert(found, packed(x, y))
        end
    end
    local x, y = pick(found)
    if x == nil then
        require, forbid = panels.step_condition(navi)
        x, y = before_enemies(navi, 1, require, forbid)
    end
    if x == nil then
        require, forbid = panels.step_condition(navi)
        x, y = before_enemies(navi, 0xFF, require, forbid)
    end
    if x == nil then
        return settle(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- 0x0802A2C8 (class 7 out of the operation battle): with the custom gauge's damage (`sub_8010B78`) 100 or more, a step to
-- a panel from which reach list 10 reaches the target; else not ready and a step anywhere.
local function gauge_reach(navi: Object, s: State)
    if battle.gauge_damage(navi.alliance) >= 100 then
        local t = target_of(s)
        local require, forbid = panels.step_condition(navi)
        local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, 10, navi.alliance)
        if x ~= nil then
            return go(navi, s, 4, x, y :: number)
        end
    end
    settle(navi, s)
end

-- VarSwrd's and NeoVari's swings' reach lists by the side's pick (0x0802A3C8: VarSwrd's six from byte 0, NeoVari's
-- four from byte 6; then the table's padding and the next routine's first byte, which a pick past a row reads).
local SWING_REACH: { number } = { 8, 3, 7, 10, 22, 6, 15, 22, 10, 6, 0, 0, 0, 0xB5 }

-- Class 8's chip: NeoVari (chip 0xD9, which 0x0802A342 and 0x0802A360 test), or any other (VarSwrd's row).
local function neovari(chip: Chip): boolean
    return chip.id == "neovari"
end

-- 0x0802A330 (class 8, VarSwrd's and NeoVari's): the side's sword pick (`sub_802E070` +0x12, which the sword reads)
-- drawn at random among the chip's swings (six; NeoVari's four).
local function swing_pick(navi: Object, s: State)
    local n = if neovari(chip_of(s)) then 4 else 6
    battle.set_sword_pick(navi.alliance, battle.rng_positive() % n)
    s.place_step = 4
    s.place_step_init = 0
end

-- 0x0802A352 (class 8's second step): the swing reaches the target from where the navi stands (as it faces): in
-- place if a hit would land, else a step anywhere; else a step to a panel from which it reaches (the third step
-- next: in place), or the next target and a step anywhere.
local function swing_reach(navi: Object, s: State)
    local at = (if neovari(chip_of(s)) then 6 else 0) + battle.sword_pick(navi.alliance)
    local range = SWING_REACH[at + 1]
    if range == nil then
        error(string.format("the side's sword pick %d reads past the table at 0x0802A3C8", battle.sword_pick(navi.alliance)))
    end
    local t = target_of(s)
    if reaches(navi, t, range, panels.front_direction(navi)) then
        if hittable(t) then
            return in_place(s)
        end
        return settle(navi, s)
    end
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, range, navi.alliance)
    if x == nil then
        return give_up(navi, s)
    end
    go(navi, s, 8, x, y :: number)
end

-- Panel conditions by side: class 9's (0x0802A448: an enemy in the side's area), 13's (0x0802A608: a panel of the
-- side's area no body stands on), 15's and 18's (0x0802A744: the side's area).
local ENEMY_IN_AREA: { { number } } = { { 0x04000000, 0x20 }, { 0x08000020, 0 } }
local HOLE_IN_AREA: { { number } } = { { 0, 0x30 }, { 0x20, 0x10 } }
local OWN_AREA: { { number } } = { { 0, 0x20 }, { 0x20, 0 } }

local function any_panel(conds: { { number } }, side: number): boolean
    local c = conds[bit32.band(side, 1) + 1]
    return #panels.filtered(c[1], c[2]) > 0
end

-- 0x0802A3F0 (class 9): an enemy stands in the navi's area: in place, else not ready and a step anywhere.
local function enemy_in_area(navi: Object, s: State)
    not_ready(s)
    if any_panel(ENEMY_IN_AREA, navi.alliance) then
        s.place_ready = true
        return
    end
    settle(navi, s)
end

-- 0x0802A474 (class 10): in place at once (the step 4 its second, in place again).
local function at_once(_navi: Object, s: State)
    in_place(s)
    s.place_step, s.place_step_init = 4, 0
end

-- 0x0802A4DA: of the panels three, two and one behind the target (toward the navi, as it faces), the first that
-- meets the conditions; none: nil.
local function behind_target(navi: Object, t: Object, require: number, forbid: number): (number?, number?)
    local back = -panels.front_direction(navi)
    for _, n in { 3, 2, 1 } do
        local x, y = t.panel_x + back * n, t.panel_y
        if field.check(x, y, require, forbid) then
            -- (A panel in column 0 would read as none: 0x0802A4B0 tests x. None is on the field.)
            return x, y
        end
    end
    return nil, nil
end

-- 0x0802A4A4 (class 11): a step to the farthest of the three panels before the target it may step onto; else the
-- next target and a step anywhere.
local function before_target(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local x, y = behind_target(navi, target_of(s), require, forbid)
    if x == nil then
        return give_up(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- 0x0802A568 (class 12): the target's counter window is open (its collision record's +0x0D): ready if a hit would
-- land; else the next target and a step anywhere (ready as it was).
local function counter_open(navi: Object, s: State)
    s.place_busy = false
    local t = target_of(s)
    if t.collision.counter_timer ~= 0 then
        if hittable(t) then
            s.place_ready = true
        end
        return
    end
    place.skip_target(s)
    wander(navi, s)
end

-- 0x0802A5BC (class 13): a panel of the navi's area no body stands on: in place, else not ready and a step anywhere.
local function hole_in_area(navi: Object, s: State)
    s.place_busy = false
    if any_panel(HOLE_IN_AREA, navi.alliance) then
        s.place_ready = true
        return
    end
    s.place_ready = false
    wander(navi, s)
end

-- 0x0802A634 (class 14): a panel it may step onto whose panel three ahead is on the field, one a body stands on
-- (0x10) and free of bodies and the side flags (0x0F880080 clear): a step there; else not ready and a step anywhere.
local function three_ahead_open(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local ahead = panels.front_direction(navi) * 3
    local found = {}
    for _, p in panels.filtered(require, forbid) do
        local x, y = bit32.band(p, 7), bit32.rshift(p, 4)
        local flags = field.flags(x + ahead, y)
        if field.valid(x + ahead, y) and bit32.band(flags, STANDABLE) ~= 0 and bit32.band(flags, 0x0F880080) == 0 then
            table.insert(found, packed(x, y))
        end
    end
    local x, y = pick(found)
    if x == nil then
        return settle(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- The area a side holds at the start, in panels, by the stage's panel column pattern (0x0802A754: patterns 56, 48,
-- 60, 49, 35 and 51 at offsets 0, 2, 6, 8, 10 and 12 of 0x0802A790, any other at 0; the side's byte at the offset).
-- Pattern 51's side 1 reads past the table into the next routine's first byte (0xB5).
local AREA_BYTES: { number } = { 9, 9, 12, 6, 6, 12, 9, 9, 9, 9, 6, 12, 0, 0xB5 }
local AREA_OFFSETS: { [number]: number } = { [56] = 0, [48] = 2, [60] = 6, [49] = 8, [35] = 10, [51] = 12 }

local function start_area(side: number): number
    local offset = AREA_OFFSETS[battle.panel_pattern()] or 0
    return AREA_BYTES[offset + side + 1]
end

-- 0x0802A728: the panels of the side's area now.
local function area_now(side: number): number
    local c = OWN_AREA[bit32.band(side, 1) + 1]
    return #panels.filtered(c[1], c[2])
end

-- 0x0802A6F0 (class 15): the navi's area is smaller than at the start: in place, else not ready and a step anywhere.
local function area_lost(navi: Object, s: State)
    s.place_busy = false
    if area_now(navi.alliance) < start_area(navi.alliance) then
        s.place_ready = true
        return
    end
    s.place_ready = false
    wander(navi, s)
end

-- 0x0802A7CA: a panel of the side's area (rows 1 to 3, columns 1 to 6) other than holy, normal or missing.
local function changed_panel(side: number): boolean
    for y = 1, 3 do
        for x = 1, 6 do
            local p = field.panel(x, y)
            if p and p.alliance == side and p.kind ~= "holy" and p.kind ~= "normal" and p.kind ~= "missing" then
                return true
            end
        end
    end
    return false
end

-- 0x0802A79C (class 16): a panel of the navi's area is of another type: in place, else not ready and a step anywhere.
local function panel_changed(navi: Object, s: State)
    s.place_busy = false
    if changed_panel(navi.alliance) then
        s.place_ready = true
        return
    end
    s.place_ready = false
    wander(navi, s)
end

-- 0x0802A804 (class 18): the navi's area is as large as at the start or larger: in place, else not ready and a step
-- anywhere.
local function area_whole(navi: Object, s: State)
    s.place_busy = false
    if area_now(navi.alliance) >= start_area(navi.alliance) then
        s.place_ready = true
        return
    end
    s.place_ready = false
    wander(navi, s)
end

-- A condition class: in place when `test` holds, else not ready and a step anywhere (classes 19 to 21, 23 to 26,
-- 29 to 31).
local function when(test: (navi: Object) -> boolean): Step
    return function(navi: Object, s: State)
        s.place_busy = false
        if test(navi) then
            s.place_ready = true
            return
        end
        s.place_ready = false
        wander(navi, s)
    end
end

-- 0x0802A83C (class 19): the navi is neither invisible nor submerged (its flags 6).
local function visible(navi: Object): boolean
    return not (navi:status("invisible") or navi:status("submerged"))
end

-- 0x0802A86C (class 20): the navi has no barrier (its collision record's +0x06).
local function unbarriered(navi: Object): boolean
    return navi.collision.barrier == "none"
end

-- 0x0802A8CA (class 21): the other side's first alive actor has a barrier. (The loop means the four, but it steps
-- the record it read, not the list: it reads the first slot four times.)
local function enemy_barrier(navi: Object): boolean
    local e = battle.alive_actor_slot(bit32.bxor(navi.alliance, 1), 1)
    return e ~= nil and e.collision.barrier ~= "none"
end

-- 0x0802A93A: a panel it may step onto whose panel ahead (as it faces) is on the field, of its side and not holy.
local function own_ahead(navi: Object, require: number, forbid: number): (number?, number?)
    local dir = panels.front_direction(navi)
    local found = {}
    for _, p in panels.filtered(require, forbid) do
        local x, y = bit32.band(p, 7), bit32.rshift(p, 4)
        local ahead = field.panel(x + dir, y)
        if field.valid(x + dir, y) and ahead ~= nil and ahead.alliance == navi.alliance and ahead.kind ~= "holy" then
            table.insert(found, packed(x, y))
        end
    end
    return pick(found)
end

-- 0x0802A90C (class 22): a step to a panel whose panel ahead is the navi's own and not holy; else not ready and a
-- step anywhere.
local function own_panel_ahead(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local x, y = own_ahead(navi, require, forbid)
    if x == nil then
        return settle(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- The element traps (chips 0x8C to 0x8F), which class 23 may set over one another.
local ELEMENT_TRAPS: { [string]: boolean } = {
    ["antifire"] = true,
    ["antiwatr"] = true,
    ["antielec"] = true,
    ["antiwood"] = true,
}

-- 0x0802A9B4 (class 23, the traps): the navi's side holds no defensive chip (`sub_802CE78`), or an element trap.
local function trap_free(navi: Object): boolean
    local held = battle.linked(navi.alliance).chip
    return held == nil or ELEMENT_TRAPS[held.id] == true
end

-- 0x0802A9F4, 0x0802AA34, 0x0802AA74 (classes 24 to 26): either side holds a sea panel; a lava panel; a broken or a
-- missing one.
local function any_sea(_navi: Object): boolean
    return count_type("sea", 0) ~= 0 or count_type("sea", 1) ~= 0
end
local function any_lava(_navi: Object): boolean
    return count_type("lava", 0) ~= 0 or count_type("lava", 1) ~= 0
end
local function any_hole(_navi: Object): boolean
    return count_type("broken", 0) ~= 0 or count_type("broken", 1) ~= 0 or count_type("missing", 0) ~= 0
        or count_type("missing", 1) ~= 0
end

-- The panels that stop class 27's walk (0x0802ABE0: bodies, by side).
local WALK_STOP: { number } = { 0x0D800000, 0x0E800000 }

-- 0x0802AB82: from (x, y) on along `dir`, until a panel off the field or one a body stands on, the panels that meet
-- the conditions.
local function walk(navi: Object, x: number, y: number, dir: number, require: number, forbid: number, out: { number })
    local stop = WALK_STOP[bit32.band(navi.alliance, 1) + 1]
    while field.valid(x, y) and bit32.band(field.flags(x, y), stop) == 0 do
        if field.check(x, y, require, forbid) then
            table.insert(out, packed(x, y))
        end
        x += dir
    end
end

-- 0x0802AB1E: each row's panels from the back of the navi's area (as it faces) forward until a body: one at random;
-- none: anywhere of its own area.
local function back_rows(navi: Object, require: number, forbid: number): (number, number)
    local dir = -panels.front_direction(navi)
    local start = if dir < 0 then 6 else 1
    local found = {}
    for y = 1, 3 do
        walk(navi, start, y, dir, require, forbid, found)
    end
    local x, y = pick(found)
    if x == nil then
        return panels.random_own(navi)
    end
    return x, y :: number
end

-- 0x0802AAE8 (class 27): a step to a panel at the back of its area. (The search always finds one: the branch for none,
-- the next target and a step anywhere, tests a column of 0.)
local function to_back(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local x, y = back_rows(navi, require, forbid)
    if x == 0 then
        return give_up(navi, s)
    end
    go(navi, s, 4, x, y)
end

-- NaviStats +0x2A, which class 28 reads first. Its EXE5 meaning isn't read (docs/design/exe5-map.md §15.1, the
-- NaviStats bytes); a netbattle's is 0 (exe5-compat reads it as 0).
local NAVI_STATS_2A = 0

-- 0x0802AC04 (class 28): with NaviStats +0x2A 0 and less than 300 HP lost, not ready (no step); else the chip reaches
-- the target from where the navi stands (as it faces): in place next; else a step to a panel from which it reaches,
-- or the next target and a step anywhere.
local function hurt_reach(navi: Object, s: State)
    if NAVI_STATS_2A == 0 and navi.max_hp - navi.hp < 300 then
        return not_ready(s)
    end
    local t, range = target_of(s), record(chip_of(s)).range
    if reaches(navi, t, range, panels.front_direction(navi)) then
        s.place_step, s.place_step_init = 4, 0
        return
    end
    local require, forbid = panels.step_condition(navi)
    local x, y = panels.reaching(t.panel_x, t.panel_y, require, forbid, range, navi.alliance)
    if x == nil then
        return give_up(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- 0x0802AC80 (class 29): the navi's side has a NaviCust bug (`sub_800FE52`).
local function bugged(navi: Object): boolean
    return battle.navi(navi.alliance).bug_kinds ~= 0
end

-- 0x0802ACDE (class 30): a field object is registered (BattleState +0xA0's eight slots).
local function any_field_object(_navi: Object): boolean
    return #field.all_objects() > 0
end

-- 0x0802AD2E (class 31): of the panels an enemy stands on (but the navi's own: side 0's flag 0x04000000, side 1's
-- the word at 0x0802ADA4, the next routine's code, its table read with the stride 16), each free panel ahead of it
-- (as the navi faces) until one isn't (0x0F800000 clear): any.
local ENEMY_ROWS: { number } = { 0x04000000, 0x58097838 }

local function free_ahead_of_enemies(navi: Object): boolean
    local dir = panels.front_direction(navi)
    local list = panels.filtered(ENEMY_ROWS[bit32.band(navi.alliance, 1) + 1], 0, navi)
    local n = 0
    for i = #list, 1, -1 do
        local x, y = bit32.band(list[i], 7), bit32.rshift(list[i], 4)
        local k = 1
        while field.check(x + dir * k, y, 0, 0x0F800000) do
            n += 1
            k += 1
        end
    end
    return n ~= 0
end

-- 0x0802ADF2: the panels it may step onto with any of the special panel bits (0x20E00; 0x21E00 with FloatShoes on).
local function special_panel(navi: Object, require: number, forbid: number): (number?, number?)
    local mask = if navi:status("float_shoe") then 0x21E00 else 0x20E00
    local found = {}
    for x = 1, 6 do
        for y = 1, 3 do
            if field.check(x, y, require, forbid) and bit32.band(field.flags(x, y), mask) ~= 0 then
                table.insert(found, packed(x, y))
            end
        end
    end
    return pick(found)
end

-- 0x0802ADBC (class 32): a step to such a panel; else the next target and a step anywhere.
local function to_special_panel(navi: Object, s: State)
    local require, forbid = panels.step_condition(navi)
    local x, y = special_panel(navi, require, forbid)
    if x == nil then
        return give_up(navi, s)
    end
    go(navi, s, 4, x, y :: number)
end

-- 0x08029D7E, 0x08029F34, 0x0802A664 (the classes' last steps): in place.
local function done(_navi: Object, s: State)
    in_place(s)
end

-- A two-step class: its first step, then in place.
local function steps(first: Step): { [number]: Step }
    return { [0] = first, [4] = done }
end

-- The classes (0x08029B68): their steps by the step byte, or one routine.
local CLASSES: { [number]: { [number]: Step } | Step } = {
    [0] = steps(reach_from_here),
    [1] = steps(front),
    [2] = spot,
    [3] = steps(landing_ahead),
    [4] = steps(behind),
    [5] = steps(three_ahead_near_enemy),
    [6] = steps(lava_ahead),
    -- [7]: below (0x0802A2A0).
    [8] = { [0] = swing_pick, [4] = swing_reach, [8] = done },
    [9] = steps(enemy_in_area),
    [10] = steps(at_once),
    [11] = steps(before_target),
    [12] = counter_open,
    [13] = steps(hole_in_area),
    [14] = steps(three_ahead_open),
    [15] = area_lost,
    [16] = panel_changed,
    [17] = steps(reach_from_here),
    [18] = area_whole,
    [19] = when(visible),
    [20] = when(unbarriered),
    [21] = when(enemy_barrier),
    [22] = steps(own_panel_ahead),
    [23] = when(trap_free),
    [24] = when(any_sea),
    [25] = when(any_lava),
    [26] = when(any_hole),
    [27] = steps(to_back),
    [28] = steps(hurt_reach),
    [29] = when(bugged),
    [30] = when(any_field_object),
    [31] = when(free_ahead_of_enemies),
    [32] = steps(to_special_panel),
}

-- Run class `class`'s step.
local function run(class: number, navi: Object, s: State)
    local c = CLASSES[class]
    if c == nil then
        error(string.format("the auto-battling navi's chip has positioning class %d, past the table at 0x08029B68", class))
    end
    if type(c) == "function" then
        return (c :: Step)(navi, s)
    end
    local step = (c :: { [number]: Step })[s.place_step]
    if step == nil then
        error(string.format("positioning class %d's step %#x reads past its table", class, s.place_step))
    end
    step(navi, s)
end

-- 0x0802A2A0 (class 7): in the operation battle class 0's; else its own steps (0x0802A2C0).
local CLASS_7: { [number]: Step } = steps(gauge_reach)

-- 0x08029B3C: one decision's step: busy until the class's routine says otherwise; whether it still is.
function place.step(navi: Object, s: State): boolean
    s.place_busy = true
    local class = record(chip_of(s)).class
    if class == 7 then
        if operation_battle() then
            run(0, navi, s)
        else
            local step = CLASS_7[s.place_step]
            if step == nil then
                error(string.format("positioning class 7's step %#x reads past its table (0x0802A2C0)", s.place_step))
            end
            step(navi, s)
        end
    else
        run(class, navi, s)
    end
    return s.place_busy
end

-- For tools (the rules' `unplayable_in_auto_battle`), never the simulation: the chips of the AI's data it has no way to get
-- a navi in place for, by id, each with why. A positioning class past the classes' table (0x08029B68 has no entry for
-- it: the original jumps wherever the memory after the table says, and crashes; `run` raises there).
function place.unplayable(): { [string]: string }
    local out: { [string]: string } = {}
    for id, r in data.chips do
        if r.class ~= 7 and CLASSES[r.class] == nil then
            out[id] = string.format(
                "the original can't play it in auto battle (positioning class %d is past the game's table of them: the game crashes)",
                r.class
            )
        end
    end
    return out
end

return place
