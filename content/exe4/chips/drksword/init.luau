--!strict
-- DrkSword (EXE4 chip 0x0BC; its record at 0x080197EC + 0xBC * 0x2C): a dark chip, which no folder holds and the
-- custom screen offers (rules/dark_chips). The swords' action 0x0A, its variant 6: a dark big slash over the two
-- columns ahead (region 0x11, the region word 0x1105FF11: hits navis, row 0x11 its own), hit modifier 3, the effect
-- table's row 0x1E, the swing's sound 0x8E. Its damage is the opponent's HP, at most 500 (its power 1000: damage
-- formula 0, 0x0801944C), worked out again every tick it waits in the hand (flag 0x80). Its cost: MegaMan steps right
-- with no key held (rules/dark_chips).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local drksword: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = { formula = "opponent_hp" },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "dark", "variable_damage" },
    action = swords.slash {
        id = "drksword/action",
        blade = swords.blade,
        hit = { region = regions.wide_long, collision = { hits = collision.hits_navis, is = collision.collision_11 }, hit_mod = 3 },
        effect = swords.effects.dark,
        sound = swords.swing,
    },
}

return { drksword = drksword }
