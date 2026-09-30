--!strict
-- TenguCross's B+Back special: weapon routine 0x10 (`sub_8011CA8`), which
-- names action 0x1C (the instant chips' action) with subtype 0x14 and a
-- 40-tick cooldown before the next B+Back. Its effect, instant chip effect
-- 0x14 (`sub_80CD4AC`), blows a gust (objects/gust) down each row from
-- the far column toward the navi's own side; the navi then waits 8 ticks
-- (action 0x1C's wait for this subtype).

local gust = require("../../../../objects/gust/gust")

local tengu_wind = {}

local ACTION = 0x1C
local SUBTYPE = 0x14
local COOLDOWN = 0x28

-- The gusts' first parameter: the weak push, turned toward the navi.
local GUST_KIND = 1

-- `sub_8011CA8`.
function tengu_wind.setup(navi: Object): number
    navi.variant = SUBTYPE
    navi.attack_lockout = COOLDOWN
    return ACTION
end

-- `sub_80CD4AC`: a gust in each row, from column 6 when the navi faces
-- right (1 when it faces left). The gusts' side is the navi's shifted by
-- its flip (the game's `lsl`), their flip 0; no damage, nothing tracks
-- them.
function tengu_wind.instant_chip(user: Object, _spec: InstantChipSpec)
    local facing = bit32.bxor(user.alliance, user.flip)
    local x = 5 * bit32.bxor(facing, 1) + 1
    local flip = user.flip
    local word = if flip < 32 then bit32.lshift(user.alliance, flip) else 0
    local side, side_flip = bit32.band(word, 0xFF), bit32.band(bit32.rshift(word, 8), 0xFF)
    for y = 1, 3 do
        gust.spawn(x, y, side, side_flip, { GUST_KIND, 0, 0, 0 }, 0, nil, 0)
    end
    battle.play_sound(0xAD)
end

return tengu_wind
