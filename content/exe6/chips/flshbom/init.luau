--!strict
-- FlshBom1-3: a FlashBomb (bomb.luau) set down three panels ahead that
-- flashes the other side's area a minute later; each level holds more HP,
-- blinds longer, and FlshBom3 hits again as it goes. The navi holds it in
-- the level's colors (its palette plus three a level).

local flash_bomb = require("@self/bomb")
local lockon = require("../rules/lockon")
local statuses = require("../rules/status")
local throw = require("../lib/bombs/throw")

-- A level's throw (`byte_80D9A20`'s row).
local function level(palette: number, variant: FlashBombVariant): Action
    return throw.action {
        held = flash_bomb.held,
        held_anim = 2,
        held_palette = palette,
        thrower = flash_bomb.thrower(flash_bomb.variant(variant)),
    }
end

return {
    flshbom1 = define.chip {
        id = "flshbom1",
        codes = { "J", "L", "Q", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 30, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.stay },
        action = level(0, { ticks = 0x3C, hp = 0x0A, status = statuses.paralyze_90, final_status = nil, palette = 0x0 }),
    },
    flshbom2 = define.chip {
        id = "flshbom2",
        codes = { "G", "K", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 34, damage = 70, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.stay },
        action = level(3, { ticks = 0x3C, hp = 0x0F, status = statuses.paralyze_150, final_status = nil, palette = 0x3 }),
    },
    flshbom3 = define.chip {
        id = "flshbom3",
        codes = { "H", "P", "S" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 38, damage = 100, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.stay },
        action = level(6, { ticks = 0x3C, hp = 0x14, status = statuses.paralyze_150, final_status = statuses.blind_480, palette = 0x6 }),
    },
}
