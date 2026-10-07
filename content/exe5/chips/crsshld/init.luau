--!strict
-- CrsShld1, CrsShld2 and CrsShld3: the crsshld series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local cross = require("@self/cross")
local guard = require("../lib/guard")

local CROSS_SHIELD = asset.sprite("cross-shield")

-- A chip's counter: a cross from the panel ahead in its palette (twice the original's parameter), after which the
-- guard ends at once.
local function counter(palette: number): (me: Object, x: number, y: number, damage: number) -> boolean
    return function(me: Object, x: number, y: number, damage: number): boolean
        cross.spawn(me, x, y, me.attack_element, damage, palette)
        return true
    end
end

-- Their shields (rows 4 to 6 of 0x080CA1F0) and counters.
local SHIELD1 = guard.look { sprite = CROSS_SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 0 }
local SHIELD2 = guard.look { sprite = CROSS_SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 2 }
local SHIELD3 = guard.look { sprite = CROSS_SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 4 }
local CROSS1, CROSS2, CROSS3 = counter(0), counter(2), counter(4)

-- CrsShld1 (EXE5 chip 0x0a9): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: EXE5's guard (lib/guard, action 0x1F with subtype 5).
local crsshld1: Chip = {
    codes = { "B", "F", "V" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 130,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = SHIELD1, counter = CROSS1 },
    auto_battle = { class = "at_once" },
}

-- CrsShld2 (EXE5 chip 0x0aa): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: EXE5's guard (lib/guard, action 0x1F with subtype 6).
local crsshld2: Chip = {
    codes = { "C", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = SHIELD2, counter = CROSS2 },
    auto_battle = { class = "at_once" },
}

-- CrsShld3 (EXE5 chip 0x0ab): the navi holds up a shield for 60 ticks; the first hit it blocks from the front sends a
-- cross along the row that strikes the first navi it meets: EXE5's guard (lib/guard, action 0x1F with subtype 7).
local crsshld3: Chip = {
    codes = { "D", "O", "Z" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 190,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    action = guard.action { ticks = 60, look = SHIELD3, counter = CROSS3 },
    auto_battle = { class = "at_once" },
}

return { crsshld1 = crsshld1, crsshld2 = crsshld2, crsshld3 = crsshld3 }
