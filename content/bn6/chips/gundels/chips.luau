--!strict
-- GunDelS1-3 and GunDelEX: a sun beam that drains the column two panels
-- ahead, more in the sun (./action, ./beam). Each level fires longer and
-- holds its gun in its own colours; GunDelEX's beam is wider and covers
-- the column past it too.

local action = require("./action")
local beam = require("./beam")
local lockon = require("../../rules/lockon")

-- The beams (the original's looks and palettes): in the shade, and in the
-- sun.
local BEAM: SunBeam = { sprite = beam.sprites.plain, palette = 0 }
local BEAM_IN_SUN: SunBeam = { sprite = beam.sprites.plain, palette = 2 }
local WIDE_BEAM: SunBeam = { sprite = beam.sprites.wide, palette = 0 }
local WIDE_BEAM_IN_SUN: SunBeam = { sprite = beam.sprites.wide, palette = 3 }

return {
    define.chip {
        id = "bn6:gundels1",
        codes = { "C", "M", "T", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 15, damage = 0, hit_param = 0, lockout = 0,
        flags = { "standard_library", "library" },
        library = { number = 15, index = 15, sort = 227 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = action.action {
            gun = action.gun(0),
            firing_ticks = 60,
            beam = BEAM,
            beam_in_sun = BEAM_IN_SUN,
        },
    },
    define.chip {
        id = "bn6:gundels2",
        codes = { "B", "E", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 30, damage = 0, hit_param = 0, lockout = 0,
        flags = { "standard_library", "library" },
        library = { number = 16, index = 16, sort = 228 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = action.action {
            gun = action.gun(3),
            firing_ticks = 90,
            beam = BEAM,
            beam_in_sun = BEAM_IN_SUN,
        },
    },
    define.chip {
        id = "bn6:gundels3",
        codes = { "N", "Q", "W" },
        element = "null", family = "null", class = "standard",
        rarity = 3, mb = 38, damage = 0, hit_param = 0, lockout = 0,
        flags = { "standard_library", "library" },
        library = { number = 17, index = 17, sort = 229 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundels1 },
        action = action.action {
            gun = action.gun(6),
            firing_ticks = 120,
            beam = BEAM,
            beam_in_sun = BEAM_IN_SUN,
        },
    },
    -- (The record's extra flag 0x01 has no battle reader, and no name.)
    -- GunDelEX's record is the Japanese games' (the content's JP-content
    -- chips take theirs): a Standard chip with the library flag; the US
    -- games' is no folder chip, with no flags.
    define.chip {
        id = "bn6:gundelex",
        codes = { "G" },
        element = "null", family = "null", class = "standard",
        rarity = 4, mb = 80, damage = 0, hit_param = 0, lockout = 0,
        flags = { "library" },
        library = { number = 201, index = 1, sort = 67 },
        slot_in_limit = 1,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.gundelex },
        action = action.action {
            gun = action.gun(9),
            firing_ticks = 120,
            beam = WIDE_BEAM,
            beam_in_sun = WIDE_BEAM_IN_SUN,
            wide = true,
        },
    },
}
