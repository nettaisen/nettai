--!strict
-- Spreadr1-3: a bullet from the panel ahead that bursts over the 3x3
-- around what it finds (action.luau). (SpoutCross's charged shot composes
-- the same action with its water cannon: navis/megaman/forms/
-- spoutcross/charge.)

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")
local effects = require("../lib/effects")
local spreader = require("@self/action")
local lockon = require("../rules/lockon")
local regions = require("@exelib/regions")

-- 16 pixels up (`dword_80ECC5C`'s rows 0, 1 and 3).
local HIGH = 0x10_0000
-- The bullet (its row 3): a flash over the 3x3 around the panel found.
-- Their gun, at the navi's attach point 11.
local GUN = attachment.look { sprite = asset.sprite("spreader"), palette = 0, attach_point = 11 }
local BULLET = bullet.variant { region = regions.square, hit_mod = 0, effect = effects.flash, sound = asset.sound("h-burst") }

local function spreadr(id: string): Action
    return spreader.action { id = id, gun = GUN, z = HIGH, bullet = BULLET }
end

local spreadr1: Chip = {
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
    slot_in_limit = 3,
    action = spreadr("spreadr1/action"),
}

local spreadr2: Chip = {
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
    slot_in_limit = 3,
    action = spreadr("spreadr2/action"),
}

local spreadr3: Chip = {
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
    slot_in_limit = 3,
    action = spreadr("spreadr3/action"),
}

return { spreadr1 = spreadr1, spreadr2 = spreadr2, spreadr3 = spreadr3 }
