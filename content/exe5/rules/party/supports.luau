--!strict
-- EXE5 DS's party supports (docs/design/exe5ds-map.md §4.7): what a benched team navi does for the navi on the field,
-- each the navi's own module's (navis/*/support), and what they share here. The battle flow calls them every fight tick
-- (0x02003764, from `sub_80080D2`'s counterpart: the rules' `fight_tick`), side 0 then side 1, in a link battle for
-- both sides and offline for side 0 alone; each tests the party block, MegaMan's light test, its member (available and
-- benched), then aims and sends its support actor (lib/party_support).

local party = require("../party")

local supports = {}

-- Where an enemy may be met: an offset list's groups (each { dx, dy, flip } entries, dx toward the enemy's front), the
-- first group with a panel that passes.
export type Offsets = { { { number } } }

-- A side's level (0x0203C0AC: the init exchange's, 0x021B82B0), which the supports' damage rows are read at.
function supports.level(side: number): number
    return party.level(side) or party.LEVEL
end

-- A support's damage word: its row's damage at the side's level, its hit flags in the high half.
function supports.damage(side: number, row: { number }, flags: number): number
    return bit32.bor(row[supports.level(side) + 1], bit32.lshift(flags, 16))
end

-- 0x0200430C: the panels' distance (Manhattan).
local function distance(x0: number, y0: number, x1: number, y1: number): number
    return math.abs(x1 - x0) + math.abs(y1 - y0)
end

-- 0x020038E4: a panel around `enemy` (dx by its facing) of the first group of `offsets` with any that passes
-- (`require`, `forbid`), one of those drawn with RNG2; and the flip the support takes there (the last entry's of the
-- group, its flip byte against the enemy's flip). Nil when the enemy isn't on the field or no group has one.
function supports.around(enemy: Object, offsets: { { { number } } }, require: number, forbid: number): (number?, number?, number)
    local ex, ey = enemy.panel_x, enemy.panel_y
    if not field.valid(ex, ey) then
        return nil, nil, 0
    end
    local dir = enemy.facing
    for _, group in offsets do
        local found: { number } = {}
        local flip = 0
        for _, e in group do
            flip = bit32.bxor(e[3], enemy.flip)
            local x, y = ex + e[1] * dir, ey + e[2]
            if field.check(x, y, require, forbid) then
                table.insert(found, x + y * 0x10)
            end
        end
        if #found > 0 then
            local pick = found[battle.rng_positive() % #found + 1]
            return bit32.band(pick, 7), bit32.rshift(pick, 4), flip
        end
    end
    return nil, nil, 0
end

-- 0x02003864 (`farthest`: 0x02003D28): of the enemies on the other side's list of alive actors, in slot order, each
-- with a panel around it (`supports.around`, a draw for each), the one nearest `navi` (the first of a tie; the farthest,
-- the first of a tie, over 0); its panel and flip.
function supports.target(navi: Object, offsets: { { { number } } }, require: number, forbid: number, farthest: boolean?): (number?, number?, number)
    local best = if farthest then 0 else 0xFF
    local bx: number?, by: number?, bflip = nil, nil, 0
    local other = 1 - navi.alliance
    for i = 1, 4 do
        local enemy = battle.alive_actor_slot(other, i)
        if enemy then
            local x, y, flip = supports.around(enemy, offsets, require, forbid)
            if x and y then
                local d = distance(navi.panel_x, navi.panel_y, enemy.panel_x, enemy.panel_y)
                if (farthest and d > best) or (not farthest and d < best) then
                    best, bx, by, bflip = d, x, y, flip
                end
            end
        end
    end
    return bx, by, bflip
end

-- One of a weighted table's values by an RNG2 draw's four bits from `shift` (`positive`: `GetPositiveSignedRNG2`'s
-- draw): the DS's timers' byte tables of offsets into their value rows.
function supports.pick(values: { number }, shift: number, positive: boolean): number
    local r = if positive then battle.rng_positive() else battle.rng()
    return values[bit32.band(bit32.rshift(r, shift), 0xF) + 1]
end

return supports
