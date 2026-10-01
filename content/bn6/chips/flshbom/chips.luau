--!strict
-- FlshBom1-3: a FlashBomb (bomb.luau) set down three panels ahead that
-- flashes the other side's area a minute later; each level holds more HP,
-- blinds longer, and FlshBom3 hits again as it goes. The navi holds it in
-- the level's colours (its palette plus three a level).

local flash_bomb = require("./bomb")
local lockon = require("../../rules/lockon")
local statuses = require("../../rules/status")
local throw = require("../../lib/bombs/throw")

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
    define.chip {
        id = "flshbom1",
        name = "FlshBom1",
        description = "Thrw dlyd\nstun bomb\n3sq fwd.",
        codes = { "J", "L", "Q", "*" },
        element = "null", family = "null", class = "standard",
        rarity = 0, mb = 30, damage = 40, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 62, index = 62, sort = 210 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.stay },
        action = level(0, { ticks = 0x3C, hp = 0x0A, status = statuses.paralyze_90, final_status = nil, palette = 0x0 }),
    },
    define.chip {
        id = "flshbom2",
        name = "FlshBom2",
        description = "Thrw dlyd\nstun bomb\n3sq fwd.",
        codes = { "G", "K", "R" },
        element = "null", family = "null", class = "standard",
        rarity = 1, mb = 34, damage = 70, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 63, index = 63, sort = 211 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.stay },
        action = level(3, { ticks = 0x3C, hp = 0x0F, status = statuses.paralyze_150, final_status = nil, palette = 0x3 }),
    },
    define.chip {
        id = "flshbom3",
        name = "FlshBom3",
        description = "Thrw dlyd\nstun bomb\n3sq fwd.",
        codes = { "H", "P", "S" },
        element = "null", family = "null", class = "standard",
        rarity = 2, mb = 38, damage = 100, hit_param = 30, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 64, index = 64, sort = 212 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.stay },
        action = level(6, { ticks = 0x3C, hp = 0x14, status = statuses.paralyze_150, final_status = statuses.blind_480, palette = 0x6 }),
    },
}
