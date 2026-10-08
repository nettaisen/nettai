--!strict
-- YelCapsl, BlkCapsl, WhiCapsl, PrpCapsl and PnkCapsl (EXE5's chips 0x17C to 0x180, past the library): MeddySoul's
-- capsules (navis/megaman/forms/meddysoul/capsules: the custom screen offers two of the five a screen, 0x08023D20, and
-- mixes one into a
-- chip picked). The screen shows a capsule's icon, name and picture, and R its description; none ever reaches a hand,
-- so nothing starts the use its record names (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local cannon_chips = require("./cannon")
local cube = require("./rockcube/cube")
local poltergeist = require("./poltrgst/poltergeist")
local rock = require("../objects/rock")
local supports = require("../lib/supports/dimming")

-- YelCapsl (EXE5 chip 0x17c): the chip it is mixed into confuses (the hand's modifier 0x04). Its record's use is
-- the cannon's (action 0x14, subtype 0).
local yelcapsl: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 0,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    action = cannon_chips.cannon.action,
}

-- BlkCapsl (EXE5 chip 0x17d): the chip it is mixed into blinds (the hand's modifier 0x08). Its record's use is
-- Poltrgst's (the dimming handler's subtype 49, its first parameter 0).
local blkcapsl: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    dimming = poltergeist.hook(false),
}

-- WhiCapsl (EXE5 chip 0x17e): the chip it is mixed into paralyzes (the hand's modifier 0x02). Its record's use is
-- RockCube's controller with the ice rock (the dimming handler's subtype 20, its first parameter 3).
local whicapsl: Chip = {
    codes = {},
    element = "null",
    family = "summon",
    class = "special",
    rarity = 0,
    mb = 6,
    damage = 200,
    hit_param = 50,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    dimming = cube.places { variant = rock.variants.ice },
}

-- PrpCapsl (EXE5 chip 0x17f): the chip it is mixed into gives the HP bug (the hand's modifier 0x20). Its record's use
-- is the supports' controller bringing Rush (the dimming handler's subtype 50, its first parameter 0).
local prpcapsl: Chip = {
    codes = {},
    element = "null",
    family = "summon",
    class = "special",
    rarity = 0,
    mb = 6,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    dimming = supports.hook("rush"),
}

-- PnkCapsl (EXE5 chip 0x180): the user of the chip it is mixed into heals a tenth of its HP at the use (the hand's
-- modifier 0x10). Its record's use is the supports' controller bringing Beat (the dimming handler's subtype 50, its
-- first parameter 1).
local pnkcapsl: Chip = {
    codes = {},
    element = "null",
    family = "summon",
    class = "special",
    rarity = 0,
    mb = 6,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = {},
    slot_in_limit = 1,
    dimming = supports.hook("beat"),
}

return { yelcapsl = yelcapsl, blkcapsl = blkcapsl, whicapsl = whicapsl, prpcapsl = prpcapsl, pnkcapsl = pnkcapsl }
