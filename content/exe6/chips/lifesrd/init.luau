--!strict
-- LifeSrd: a big slash on the two columns ahead; the Program Advance of Sword, WideSwrd, LongSwrd (or
-- WideBlde, LongBlde).

local lockon = require("../rules/lockon")
local longblde = require("./longblde").longblde
local longswrd = require("./longswrd").longswrd
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")
local sword = require("./sword").sword
local wideblde = require("./wideblde").wideblde
local wideswrd = require("./wideswrd").wideswrd

-- The big slash.
local BIG_SLASH = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0 }

-- Its charged slash in SlashCross: the big slash over two columns.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.wide_long, sprite = asset.sprite("big-slash"), anim = 0 },
    blade = parts.blade,
    sound = parts.big_swing,
    lockon = lockon.drksword,
}

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.drksword },
    slot_in_limit = 1,
    program_advances = {
        { order = 0, sequence = { sword, wideswrd, longswrd } },
        { order = 1, sequence = { sword, wideblde, longblde } },
    },
    action = slash.action {
        id = "lifesrd/action",
        blade = parts.blade,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = BIG_SLASH,
        sound = parts.big_swing,
        charged = CHARGED,
    },
}

return { lifesrd = chip }
