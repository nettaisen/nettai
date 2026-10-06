--!strict
-- CustSwrd (EXE5 chip 0x036): a big slash over the two columns ahead (region 0x11, as EXE6's LifeSrd) whose damage is
-- the custom gauge's (EXE5's formula 45, 0x0800EA20: EXE6's `sub_8010B78`, the same code): the shared slash
-- (@exelib/swords/slash, EXE6's action 0x13; EXE5's the same code) with EXE5's parts (lib/swords), its slash
-- effect 0x28 and the plain swing.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local regions = require("@exelib/regions")
local swords = require("../lib/swords")

local chip: Chip = {
    codes = { "E", "L", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 20,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
    -- (The custom screen shows its damage as "???" only on an A, which it never comes as.)
    traits = { "hides_damage_as_a" },
    slot_in_limit = 3,
    damage = { formula = "gauge" },
    action = swords.slash {
        blade = swords.blade,
        blade_anim = swords.blade_anim,
        hit = { region = regions.wide_long, collision = swords.slashes_navis, hit_mod = 3 },
        effect = swords.effects.custom,
        sound = swords.swing,
    },
    auto_battle = { class = "gauge_into_reach" },
}

return { custswrd = chip }
