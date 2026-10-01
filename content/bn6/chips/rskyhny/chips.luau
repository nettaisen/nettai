--!strict
-- RskyHny1-3: a hive that sends a bee ahead, and another for each hit the
-- navi takes in the next three windows (./action, ./bee); each level's
-- bees fly faster.
--
-- RskyHny3's record stays the pack's for now (MstrCros's Program Advance
-- recipe names it by number): this module gives its action, which
-- registration by number runs (chips/027-rskyhny3; docs/design/
-- content-model-v2.md §12).

local action = require("./action")

-- The bees by level (the chips' first parameter, `byte_80D3164`): the
-- palette, and the speeds across and along the field.
local BEES: { HoneyBee } = {
    { palette = 0, speed = 0x2_8000, turn_speed = 0x1_8000 },
    { palette = 1, speed = 0x3_0000, turn_speed = 0x1_CCCC },
    { palette = 2, speed = 0x3_8000, turn_speed = 0x2_1999 },
}

return {
    define.chip {
        id = "rskyhny1",
        name = "RskyHny1",
        description = "Bees attk\nwhle hive\ndefending",
        codes = { "B", "G", "S" },
        element = "wood", family = "wood", class = "standard",
        rarity = 0, mb = 21, damage = 10, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 39, index = 39, sort = 351 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = action.action { bee = BEES[1] },
    },
    define.chip {
        id = "rskyhny2",
        name = "RskyHny2",
        description = "Bees attk\nwhle hive\ndefending",
        codes = { "C", "R", "V" },
        element = "wood", family = "wood", class = "standard",
        rarity = 1, mb = 28, damage = 15, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 40, index = 40, sort = 399 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (a number until rules/lockon.luau).
        beast = { lockon = 1 },
        action = action.action { bee = BEES[2] },
    },
    -- RskyHny3's record's action.
    { action = action.action { id = "rskyhny3/action", bee = BEES[3] } },
}
