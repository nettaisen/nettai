--!strict
-- Beast Over's berserk (`sub_802D322`), the beast part's `controller`: a
-- navi in a `controlled` form (forms 0x17 and 0x18) doesn't read the
-- player's buttons; its idle action cycles on its own through moving next
-- to an opponent, using the next chip, and firing the buster. Its state is
-- the part's (the original's last 0x10 bytes of the navi's AIData,
-- +0xF0), which the Cross special (cross-special.luau) shares and the
-- form's `berserk` effect starts over (`sub_802D310`).

-- The step conditions, an opponent's panel, what ends the look behind an
-- opponent and the opposing player's panel flag, by side.
local berserk_rules = require("../berserk")

local berserk = {}

-- The beast part's state fields the controllers keep (rules/beast/
-- init.luau declares them): the berserk's and the Cross special's.
export type State = {
    -- +0: the step (read as a jump-table offset).
    controller_step: number,
    -- +4: moves since the last chip or buster shot.
    controller_moves: number,
    -- +6: the Cross special's buster volley: shots left while an opponent
    -- stays in the row.
    controller_volley: number,
    -- +8: the controller has started (the berserk's first step is a move;
    -- the Cross special's first chip sounds).
    controller_started: boolean,
}

-- `sub_802D310`: the state starts over.
function berserk.reset(s: State)
    s.controller_step, s.controller_moves, s.controller_volley, s.controller_started = 0, 0, 0, false
end

-- Use the next chip (`sub_802D358`), fire the buster (`sub_802D3A8`), step
-- next to an opponent (`sub_802D3CA`).
local USE_CHIP, BUSTER, MOVE = 0, 1, 2
berserk.USE_CHIP, berserk.BUSTER, berserk.MOVE = USE_CHIP, BUSTER, MOVE

-- Where the lock-on marker's target stands when the navi has no marker
-- (`sub_80E1676`): off the field.
local NO_MARKER_X, NO_MARKER_Y = 0xA0, 0xE3
berserk.NO_MARKER_X, berserk.NO_MARKER_Y = NO_MARKER_X, NO_MARKER_Y

-- `sub_80E7486`: the opposing player standing on (x, y) (the panel shows
-- the other side's player flag and an alive actor's collision sits there).
function berserk.opponent_on(x: number, y: number, alliance: number): Object?
    local flag = berserk_rules.opposing_player[alliance + 1]
    if bit32.band(field.flags(x, y), flag) == 0 then
        return nil
    end
    for side = 0, 1 do
        for _, o in battle.alive_actors(side) do
            local c = o.collision :: Collision?
            if c ~= nil and c.panel_x == x and c.panel_y == y then
                return o
            end
        end
    end
    return nil
end

-- A random one of `panels` (`GetPositiveSignedRNG2` modulo their count;
-- no draw when there are none).
local function pick<T>(panels: { T }): T?
    if #panels == 0 then
        return nil
    end
    return panels[battle.rng_positive() % #panels + 1]
end

-- A panel list entry as the game keeps it: `x | y << 4` in a byte.
local function pack(x: number, y: number): number
    return bit32.bor(x, bit32.lshift(y, 4))
end

-- `object_getPanelsInRowFiltered`: the panels of row `y` meeting `cond`,
-- left to right, packed.
local function panels_in_row(y: number, cond: { require: number, forbid: number }): { number }
    local out = {}
    for x = 1, 6 do
        if field.check(x, y, cond.require, cond.forbid) then
            table.insert(out, pack(x, y))
        end
    end
    return out
end

-- `sub_810971A`: an opponent's panel in row `y`, packed, picked at random
-- among them (one RNG draw when there is one).
function berserk.opponent_in_row(navi: Object, y: number): number?
    return pick(panels_in_row(y, berserk_rules.opponent[navi.alliance + 1]))
end

-- `sub_8015CC0(x, y, cond, 0)`: from the opponent at `from` (packed), the
-- panels behind it (toward the navi's side), up to the first one with a
-- neutral object or the navi side's other bodies, that meet `cond`; one at
-- random, packed. None found: 0 if something blocked the way, else the row
-- alone (x 0, the register the scan leaves).
local function behind(navi: Object, from: number, cond: { require: number, forbid: number }): number
    local dir = if navi.alliance == 0 then -1 else 1
    local blocking = berserk_rules.blocking[navi.alliance + 1]
    local y = bit32.rshift(from, 4)
    local x = bit32.band(from, 7) + dir
    local found = {}
    local blocked: boolean
    while true do
        if bit32.band(field.flags(x, y), blocking) ~= 0 then
            blocked = true
            break
        end
        if field.check(x, y, cond.require, cond.forbid) then
            table.insert(found, pack(x, y))
        end
        x += dir
        if x < 0 or not field.valid(x, y) then
            blocked = false
            break
        end
    end
    local p = pick(found)
    if p ~= nil then
        return p :: number
    end
    return if blocked then 0 else bit32.lshift(y, 4)
end

-- `sub_802D430`: where to step. For each row with an opponent, a panel
-- behind it (toward the navi's side) that meets `cond`; if none of those
-- turns up, any panel meeting `cond` in the rows with an opponent; if none,
-- the navi's own panel. The navi's own panel counts as reserved meanwhile.
local function choose_panel(navi: Object, cond: { require: number, forbid: number }): (number, number)
    local ox, oy = navi.panel_x, navi.panel_y
    navi:reserve_panel(ox, oy)
    local entries: { number } = {}
    local count = 0
    for y = 1, 3 do
        local opponent = berserk.opponent_in_row(navi, y)
        if opponent ~= nil then
            local v = behind(navi, opponent, cond)
            table.insert(entries, v)
            if v ~= 0 then
                count += 1
            end
        end
    end
    if count == 0 then
        entries = {}
        for y = 1, 3 do
            if berserk.opponent_in_row(navi, y) ~= nil then
                for _, p in panels_in_row(y, cond) do
                    table.insert(entries, p)
                end
            end
        end
        count = #entries
    end
    local tx, ty = ox, oy
    if count ~= 0 then
        -- The draw only covers the first `count` entries, which with the
        -- first search's misses among them may include a miss.
        local v = entries[battle.rng_positive() % count + 1] :: number
        tx, ty = bit32.band(v, 7), bit32.rshift(v, 4)
    end
    navi:unreserve_panel(ox, oy)
    return tx, ty
end

-- `sub_802D358`: use the next chip, unless (for the first three moves after
-- a chip) the opponent under the lock-on marker is flashing, or no chip is
-- left or it can't start; then the buster is next.
local function use_chip(navi: Object, s: State): "nothing" | "chip"
    if s.controller_moves <= 3 then
        local x, y = NO_MARKER_X, NO_MARKER_Y
        local marker = navi.target_marker
        if marker ~= nil then
            x, y = marker.panel_x, marker.panel_y
        end
        local t = berserk.opponent_on(x, y, navi.alliance)
        if t ~= nil and t:status("flashing") then
            s.controller_step = BUSTER
            return "nothing"
        end
    end
    if navi:next_chip() == nil then
        s.controller_step = BUSTER
        return "nothing"
    end
    navi:set_request("chip", true)
    if not navi:use_chip() then
        s.controller_step = BUSTER
        return "nothing"
    end
    s.controller_moves = 0
    s.controller_step = MOVE
    return "chip"
end

-- `sub_802D3A8`: after three moves, or with an opponent in the navi's row,
-- fire the buster; a move is next either way.
local function buster(navi: Object, s: State): "nothing" | "buster"
    s.controller_step = MOVE
    if not (s.controller_moves > 2 or berserk.opponent_in_row(navi, navi.panel_y) ~= nil) then
        return "nothing"
    end
    s.controller_moves = 0
    return "buster"
end

-- `sub_802D3CA`: step to a panel near an opponent (`sub_802D430`); a chip
-- is next.
function berserk.step_toward_opponent(navi: Object, s: State): "moved"
    local conds = if navi:status("air_shoe") then berserk_rules.step.floor_free else berserk_rules.step.grounded
    local tx, ty = choose_panel(navi, conds[navi.alliance + 1])
    navi:start_move_to(tx, ty, 8)
    s.controller_moves = (s.controller_moves + 1) % 0x10000
    s.controller_step = USE_CHIP
    return "moved"
end

-- `sub_802D322`: one tick of the controller, from the idle action.
function berserk.control(side: number, navi: Object): "nothing" | "chip" | "buster" | "moved"
    local s = rules.state() :: State
    -- (`sub_802D310` cleared it: it starts over.)
    if navi.controller_fresh then
        navi.controller_fresh = false
        berserk.reset(s)
    end
    if not s.controller_started then
        s.controller_started = true
        s.controller_step = MOVE
    end
    if s.controller_step == USE_CHIP then
        return use_chip(navi, s)
    elseif s.controller_step == BUSTER then
        return buster(navi, s)
    end
    return berserk.step_toward_opponent(navi, s)
end

return berserk
