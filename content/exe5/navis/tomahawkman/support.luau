--!strict
-- TomahawkMan's party support (EXE5 DS: 0x02003DB4, his actor the support actor's variant 3, 08-0A;
-- docs/design/exe5ds-map.md §4.7): while he is benched, every 180 to 400 ticks outside the dimming he comes onto a panel
-- of his side's area from which his swing reaches an enemy, and swings his tomahawk over the two columns ahead; his timer
-- starts again as he goes (or when he can't come).

local collision = require("../../rules/collision")
local party = require("../../rules/party")
local party_support = require("../../lib/party_support")
local regions = require("@exelib/regions")
local supports = require("../../rules/party/supports")
local tomahawkman = require("../tomahawkman").tomahawkman

-- His damage by the side's level (0x020CC294), its hit flags 0x9E.
local DAMAGE: { number } = { 40, 40, 40, 40, 50, 50, 70, 80, 90, 90 }
local FLAGS = 0x9E
-- His timer (0x02003E88: RNG2 bits 6–9 into 0x020CC2B0's offsets of 0x020CC2A8's ticks, weights 2/6/6/2).
local TIMER: { number } = { 180, 180, 240, 240, 240, 240, 240, 240, 350, 350, 350, 350, 350, 350, 400, 400 }
-- By his side: the panels he searches from (0x020CC2C0: side 0 off the panels flagged 0x20, side 1 on them) and those
-- he may stand on (0x020CC2D0: solid and his side's own, nothing on it or reserved).
local SEARCH_REQUIRE = { [0] = 0, [1] = 0x20 }
local SEARCH_FORBID = { [0] = 0x20, [1] = 0 }
local STAND_REQUIRE = { [0] = 0x10, [1] = 0x30 }
local STAND_FORBID = { [0] = 0x0F8800A0, [1] = 0x0F880080 }
-- The panel patterns where he may face back too (`GetBattlePanelColumnPattern`).
local TWO_WAY = { [0x23] = true, [0x31] = true, [0x33] = true }

-- His appearance (animation 0x10, 20 ticks); his swing (animation 0x11, its sound), its hit on its 11th tick, 16 pixels
-- up, over the panel ahead's column and the one past it (region 0x11, as a slash at navis), the earth it splashes there
-- (the effect table's row 0x45); 31 ticks.
local ANIM_APPEAR, ANIM_SWING = 0x10, 0x11
local SWING_SOUND = asset.sound("tomahawk-man")
local EARTH = new.effect { sprite = asset.sprite("big-slash"), anim = 1, palette = 0 }
local HIT_AT, SWING_ENDS, SWING_Z = 0xA, 0x1E, 0x10_0000

local support = {}

-- 0x02003E88: his timer starts again.
local function reset(side: number)
    party.supports(side).tomahawkman_timer = supports.pick(TIMER, 6, true)
end

-- 0x021FEB75: the swing.
local function swing(me: Object)
    if me.phase_init == 0 then
        me.anim = ANIM_SWING
        battle.play_sound(SWING_SOUND)
        me.timer = 0
        me.phase_init = 4
        return
    end
    if me.timer == HIT_AT then
        local x, y = me.panel_x + me.facing, me.panel_y
        battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = 0,
            z = SWING_Z,
            region = regions.wide_long,
            target = collision.hits_navis,
            self_type = collision.slash,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = 0,
        })
        local cx, cy = field.center(x, y)
        battle.effect(Vec3.new(cx, cy, SWING_Z), EARTH, bit32.bxor(me.alliance, me.flip))
    end
    me.timer += 1
    if me.timer > SWING_ENDS then
        party_support.go(me, party_support.WARP)
    end
end

support.actor = party_support.make {
    id = "tomahawkman/support",
    navi = tomahawkman,
    appear_ticks = 0x14,
    appear_anim = ANIM_APPEAR,
    actions = { [party_support.ATTACK] = swing },
    gone = function(me: Object)
        reset(me.alliance)
    end,
}

-- `object_getFirstPanelInDirectionFiltered`: from column `x` stepping by `dir`, the first panel of row `y` that passes;
-- nil once off the field.
local function first(x: number, y: number, require: number, forbid: number, dir: number): number?
    while true do
        if field.check(x, y, require, forbid) then
            return x
        end
        x += dir
        if not field.valid(x, y) then
            return nil
        end
    end
end

-- 0x02004890: an enemy (the other side's list of alive actors) on the panels of region 0x11 around (x, y), facing
-- `dir`.
local function reaches(side: number, x: number, y: number, dir: number): boolean
    for i = 1, 4 do
        local enemy = battle.alive_actor_slot(1 - side, i)
        if enemy then
            for _, p in regions.wide_long.panels or {} do
                if enemy.panel_x == x + p[1] * dir and enemy.panel_y == y + p[2] then
                    return true
                end
            end
        end
    end
    return false
end

-- One of `found` ({ x, y } pairs) drawn with RNG2.
local function draw(found: { { number } }): (number?, number?)
    if #found == 0 then
        return nil, nil
    end
    local p = found[battle.rng_positive() % #found + 1]
    return p[1], p[2]
end

-- 0x02003F0C: where he may come facing `flip` (0, or 1 the other way): searched from the far column back toward his
-- side, a panel of his side's from which his swing reaches an enemy: the top and bottom rows first (2 in 16 the middle
-- row first), then the middle row, then any row from the panels he may stand on.
local function search(side: number, flip: number): (number?, number?)
    local front = if bit32.bxor(side, flip) == 0 then 1 else -1
    local back = -front
    local start = bit32.bxor(bit32.bxor(side, flip), 1) * 5 + 1
    local sreq, sforbid = SEARCH_REQUIRE[side], SEARCH_FORBID[side]
    local req, forbid = STAND_REQUIRE[side], STAND_FORBID[side]
    local tried = false
    local step = if bit32.band(battle.rng_positive(), 0xF) <= 1 then "middle" else "edges"
    while true do
        if step == "edges" then
            local found: { { number } } = {}
            for _, y in { 1, 3 } do
                local x = first(start, y, sreq, sforbid, back)
                if x then
                    table.insert(found, { x, y })
                end
            end
            local good: { { number } } = {}
            for _, p in found do
                if reaches(side, p[1] + front, p[2], front) and field.check(p[1], p[2], req, forbid) then
                    table.insert(good, p)
                end
            end
            if #good > 0 then
                return draw(good)
            end
            if tried then
                step = "any"
            else
                tried = true
                step = "middle"
            end
        elseif step == "middle" then
            local x = first(start, 2, sreq, sforbid, back)
            if x and field.check(x, 2, req, forbid) and reaches(side, x + front, 2, front) then
                return x, 2
            end
            if tried then
                step = "any"
            else
                tried = true
                step = "edges"
            end
        else
            local found: { { number } } = {}
            for y = 1, 3 do
                local x = first(start, y, req, forbid, back)
                if x and reaches(side, x + front, y, front) then
                    table.insert(found, { x, y })
                end
            end
            return draw(found)
        end
    end
end

-- 0x02003EB0: where he comes, and his flip: facing his side's way, and in the panel patterns where the areas
-- interleave facing back too; one of those found drawn with RNG2.
local function target(side: number): (number?, number?, number)
    local found: { { number } } = {}
    local flip = 0
    while true do
        local x, y = search(side, flip)
        if x and y then
            table.insert(found, { x, y, flip })
        end
        if not TWO_WAY[battle.panel_pattern()] then
            break
        end
        flip += 1
        if flip >= 2 then
            break
        end
    end
    if #found == 0 then
        return nil, nil, 0
    end
    local p = found[battle.rng_positive() % #found + 1]
    return p[1], p[2], p[3]
end

-- 0x02003DB4 (each fight tick): the light test, TomahawkMan a benched member who may support; his timer set once a
-- turn; outside the dimming, his timer runs out: the side's navi not TomahawkMan, no member busy, a panel; else his timer
-- starts again.
function support.tick(side: number)
    if not party.light(side) then
        return
    end
    local m = party.member_of(side, tomahawkman)
    if m == nil or not party.may_support(side, tomahawkman) or not party.benched(side, tomahawkman) then
        return
    end
    local s = party.supports(side)
    if not s.tomahawkman_set then
        s.tomahawkman_set = true
        reset(side)
    end
    if battle.dimmed() then
        return
    end
    s.tomahawkman_timer = bit32.band(s.tomahawkman_timer - 1, 0xFFFF)
    if s.tomahawkman_timer ~= 0 then
        return
    end
    local navi = battle.player(side)
    if navi == nil or battle.navi(side).navi == tomahawkman then
        return
    end
    local x, y, flip
    if not party.any_busy(side) then
        x, y, flip = target(side)
    end
    if x == nil or y == nil then
        reset(side)
        return
    end
    party_support.send(support.actor, navi, x, y, flip, supports.damage(side, DAMAGE, FLAGS), m)
end

return support
