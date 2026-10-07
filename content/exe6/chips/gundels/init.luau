--!strict
-- GunDelS1-3 and GunDelEX: a sun beam that drains the column two panels
-- ahead, more in the sun (./action, ./beam). Each level fires longer and
-- holds its gun in its own colors; GunDelEX's beam is wider and covers
-- the column past it too.

local action = require("@self/action")
local lockon = require("../rules/lockon")
local regions = require("@exelib/regions")

-- The beams (the original's looks, `dword_80E5C28`, and palettes): in the
-- shade, and in the sun; GunDelEX's wider.
local BEAM: SunBeam = { sprite = asset.sprite("sun-beam"), palette = 0 }
local BEAM_IN_SUN: SunBeam = { sprite = asset.sprite("sun-beam"), palette = 2 }
local WIDE_BEAM: SunBeam = { sprite = asset.sprite("sun-beam-ex"), palette = 0 }
local WIDE_BEAM_IN_SUN: SunBeam = { sprite = asset.sprite("sun-beam-ex"), palette = 3 }

local chips: { [string]: Chip } = {
    gundels1 = {
        codes = { "C", "M", "T", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 15, damage = 0, hit_param = 0, lockout = 0,
        flags = { "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = action.action {
            gun = action.gun(0),
            firing_ticks = 60,
            beam = BEAM,
            beam_in_sun = BEAM_IN_SUN,
            region = regions.wide,
        },
    },
    gundels2 = {
        codes = { "B", "E", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 0, hit_param = 0, lockout = 0,
        flags = { "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = action.action {
            gun = action.gun(3),
            firing_ticks = 90,
            beam = BEAM,
            beam_in_sun = BEAM_IN_SUN,
            region = regions.wide,
        },
    },
    gundels3 = {
        codes = { "N", "Q", "W" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 38, damage = 0, hit_param = 0, lockout = 0,
        flags = { "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = action.action {
            gun = action.gun(6),
            firing_ticks = 120,
            beam = BEAM,
            beam_in_sun = BEAM_IN_SUN,
            region = regions.wide,
        },
    },
    -- (The record's extra flag 0x01 has no battle reader, and no name.)
    -- GunDelEX's record is the Japanese games' (the content's JP-content
    -- chips take theirs): a Standard chip with the library flag; the US
    -- games' is no folder chip, with no flags.
    gundelex = {
        codes = { "G" },
        element = "null", family = "null", class = "standard",
        rarity = 4, mb = 80, damage = 0, hit_param = 0, lockout = 0,
        flags = { "library" },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundelex },
        action = action.action {
            gun = action.gun(9),
            firing_ticks = 120,
            beam = WIDE_BEAM,
            beam_in_sun = WIDE_BEAM_IN_SUN,
            region = regions.wide_long,
        },
    },
}

return chips
