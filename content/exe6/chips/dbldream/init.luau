--!strict
-- DblDream: two big slashes on the two columns ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- The big slash.
local BIG_SLASH = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0 }

-- Its charged slashes in SlashCross: two big slashes over two columns, the
-- first flinching. (The second's lock-on mode is the table's entry; the
-- rush has locked on by then.)
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.wide_long, sprite = asset.sprite("big-slash"), anim = 0, hit_mod = 1 },
    blade = parts.blade,
    sound = parts.big_swing,
    again = slashes.slash {
        wave = slashes.wave { region = regions.wide_long, sprite = asset.sprite("big-slash"), anim = 0 },
        blade = parts.blade,
        sound = parts.big_swing,
        lockon = lockon.drksword,
    },
    lockon = lockon.drksword,
}

local chip: Chip = {
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.drksword },
    action = slash.action {
        blade = parts.blade,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 1 },
        effect = BIG_SLASH,
        sound = parts.big_swing,
        swings = 2,
        charged = CHARGED,
    },
}

return { dbldream = chip }
