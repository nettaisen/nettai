--!strict
-- Spreadr1-3: a bullet from the panel ahead that bursts over the 3x3
-- around what it finds (action.luau). (SpoutCross's charged shot composes
-- the same action with its water cannon: navis/megaman/forms/
-- spoutcross/charge.)

local bullet = require("../../objects/bullet/bullet")
local spreader = require("./action")
local lockon = require("../../rules/lockon")

-- 16 pixels up (`dword_80ECC5C`'s rows 0, 1 and 3).
local HIGH = 0x10_0000

local function spreadr(id: string): Action
    return spreader.action { id = id, gun = spreader.guns.spreader, z = HIGH, bullet = bullet.variants.spreader }
end

local spreadr1 = define.chip {
    id = "spreadr1",
    name = "Spreadr1",
    description = "Spreads \ndamg to \nadj panls",
    codes = { "L", "M", "N", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 30,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 9, index = 9, sort = 417 },
    slot_in_limit = 3,
    action = spreadr("spreadr1/action"),
}

local spreadr2 = define.chip {
    id = "spreadr2",
    name = "Spreadr2",
    description = "Spreads \ndamg to \nadj panls",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 18,
    damage = 60,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 10, index = 10, sort = 418 },
    slot_in_limit = 3,
    action = spreadr("spreadr2/action"),
}

local spreadr3 = define.chip {
    id = "spreadr3",
    name = "Spreadr3",
    description = "Spreads \ndamg to \nadj panls",
    codes = { "Q", "R", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 26,
    damage = 90,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.cannon },
    library = { number = 11, index = 11, sort = 419 },
    slot_in_limit = 3,
    action = spreadr("spreadr3/action"),
}

return { spreadr1 = spreadr1, spreadr2 = spreadr2, spreadr3 = spreadr3 }
