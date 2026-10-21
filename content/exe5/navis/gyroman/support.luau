--!strict
-- GyroMan's party support (EXE5 DS: 0x02003984, his controller the DS's effect object 0x92, 0x02225DA8;
-- docs/design/exe5ds-map.md §4.7): once a battle, while he is benched, his wind blows for 400 to 660 ticks: a gust
-- (EXE5's, pulling, no damage) down a row from the back of the enemy area, the rows in turn, one every 12 ticks while
-- no wind holds the side and fewer than two of his gusts blow (in the panel patterns where the areas interleave, from
-- either edge in turn, one every 6). He leaves the bottom screen's panel lit while it lasts; it ends when he is no
-- longer benched.

local gust = require("../megaman/gust")
local party = require("../../rules/party")
local panels = require("@exelib/panels")
local gyroman = require("../gyroman").gyroman

-- How long his wind blows (0x020CC1EC's offsets of 0x020CC1FC's ticks by RNG2 bits 15–18, weights 2/6/6/2).
local LASTS: { number } = { 400, 400, 480, 480, 480, 480, 480, 480, 540, 540, 540, 540, 540, 540, 660, 660 }
-- The waits after a gust (an edge's in turn, the back's).
local EDGE_WAIT, BACK_WAIT = 6, 0xC
-- The panel patterns where the areas interleave.
local TWO_WAY = { [0x23] = true, [0x31] = true, [0x33] = true }
-- Where a gust starts (0x0222B300 by the side): the other side's area.
local AREA_REQUIRE = { [0] = 0x20, [1] = 0 }
local AREA_FORBID = { [0] = 0, [1] = 0x20 }
-- A panel's flag that holds a gust back (0x00800000), and the flag of the other side's area (0x20).
local BLOCKED, OTHER_AREA = 0x0080_0000, 0x20
local SOUND = asset.sound("dustbrk")
local BLOW, WAIT = 0, 4

type State = {
    -- Its wait after a gust (+0x22), the row it blew last (+0x6C), the edge next in the interleaved patterns (+0x74),
    -- each of its two gusts blowing (+0x60, +0x61). (How long it blows on is its timer, +0x20.)
    wait: number,
    row: number,
    edge: number,
    gusts: { number },
}

local support = {}

-- 0x02225EFC: the column a gust starts on in row `y`: the other side's area's nearest panel in the row
-- (`object_getClosestPanelMatchingRowFiltered`), on through the area to its back (to the last before a panel that holds
-- gusts back or isn't the area's); 0 if the nearest holds them back.
local function start_column(me: Object, y: number): number
    local side = me.alliance
    local x = panels.closest_in_row(bit32.bxor(bit32.bxor(side, me.flip), 1), y, AREA_REQUIRE[side], AREA_FORBID[side])
    if bit32.btest(field.flags(x, y), BLOCKED) then
        return 0
    end
    local dir = if side == 0 then 1 else -1
    local area = bit32.band(field.flags(x, y), OTHER_AREA)
    while true do
        x += dir
        if x < 1 or x > 6 then
            break
        end
        local f = field.flags(x, y)
        if bit32.btest(f, BLOCKED) or bit32.band(f, OTHER_AREA) ~= area then
            break
        end
    end
    return x - dir
end

-- 0x02225E10: a gust, when no wind holds the side and a slot is free; the next row (3, 2, 1 in turn).
local function blow(me: Object)
    local s = me.state :: State
    local side = me.alliance
    if battle.wind(side) then
        return
    end
    local slot = if s.gusts[1] == 0 then 0 elseif s.gusts[2] == 0 then 1 else nil
    if slot == nil then
        return
    end
    s.row -= 1
    if s.row == 0 then
        s.row = 3
    end
    if TWO_WAY[battle.panel_pattern()] then
        gust.spawn(s.edge * 5 + 1, s.row, side, bit32.bxor(s.edge, 1), { pulls = true }, 0, me, slot)
        s.edge = bit32.bxor(s.edge, 1)
        s.wait = EDGE_WAIT
    else
        local x = start_column(me, s.row)
        if x == 0 then
            return
        end
        gust.spawn(x, s.row, side, me.flip, { pulls = true }, 0, me, slot)
        s.wait = BACK_WAIT
    end
    me.phase = WAIT
end

-- 0x02225DD0: until its time is up, a gust and a wait in turn.
local function update(me: Object)
    local s = me.state :: State
    if me.timer ~= 0 then
        me.timer -= 1
        if me.timer == 0 then
            me:set_lifecycle("destroy")
            return
        end
    end
    if me.phase == BLOW then
        blow(me)
    elseif me.phase == WAIT then
        s.wait -= 1
        if s.wait == 0 then
            me.phase = BLOW
        end
    else
        error(string.format("GyroMan's wind: phase %#x reads past its table (0x0222B2F8)", me.phase))
    end
end

support.kind = new.kind {
    id = "gyroman/support",
    pool = "effect",
    state = { wait = "u16", row = "u8", edge = "u8", gusts = "u8[2]" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x02225DB8.
            (me.state :: State).row = 3
            battle.play_sound(SOUND)
            me:set_lifecycle("update")
            update(me)
        elseif l == "update" then
            update(me)
        else
            -- 0x02225EA0: its gusts let go, the party block's word for it cleared (and the panel's button dark).
            local s = me.state :: State
            s.gusts[1], s.gusts[2] = 0, 0
            party.supports(me.alliance).gyroman_gust = nil
            me:free()
        end
    end,
}

-- 0x02225FA4: it ends (its gusts let go).
local function remove(o: Object)
    o:set_lifecycle("destroy")
    local s = o.state :: State
    s.gusts[1], s.gusts[2] = 0, 0
end

-- 0x02003984 (each fight tick): the light test; GyroMan a member who may support and benched, his wind once a battle
-- (the party block's +2, never cleared), while none blows (+0x1C); out of the party or on the field, his wind ends.
function support.tick(side: number)
    if not party.light(side) then
        return
    end
    local s = party.supports(side)
    local m = party.member_of(side, gyroman)
    if m ~= nil then
        if not party.may_support(side, gyroman) then
            return
        end
        if party.benched(side, gyroman) then
            if s.gyroman_spawned or s.gyroman_gust then
                return
            end
            s.gyroman_spawned = true
            -- (0x02225ED0.)
            local lasts = LASTS[bit32.band(bit32.rshift(battle.rng(), 15), 0xF) + 1]
            local o = battle.spawn(support.kind)
            if o then
                o.alliance, o.flip = side, 0
                o.run_while_dimmed = false
                o.timer = lasts
                s.gyroman_gust = o
            end
            return
        end
    end
    local o = s.gyroman_gust
    if o then
        remove(o)
    end
end

return support
