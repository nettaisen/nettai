--!strict
-- Voltz1, Voltz2 and Voltz3: the voltz series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local voltz = require("@self/voltz")

-- Voltz1 (EXE5 chip 0x04e): a sensor dropped three panels ahead that shocks the panels beside it, the shocks
-- going on over the other side's panels (./voltz, instant effect 16; the record's params: level 0, the
-- shocks every 20 ticks).
local voltz1: Chip = {
    codes = { "K", "O", "Y" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 100,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = voltz.instant(0, 20),
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

-- Voltz2 (EXE5 chip 0x04f): a sensor dropped three panels ahead that shocks the panels beside it, the shocks
-- going on over the other side's panels (./voltz, instant effect 16; the record's params: level 1, the
-- shocks every 20 ticks).
local voltz2: Chip = {
    codes = { "J", "U", "Z" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 1,
    mb = 48,
    damage = 120,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = voltz.instant(1, 20),
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

-- Voltz3 (EXE5 chip 0x050): a sensor dropped three panels ahead that shocks the panels beside it, the shocks
-- going on over the other side's panels (./voltz, instant effect 16; the record's params: level 2, the
-- shocks every 20 ticks).
local voltz3: Chip = {
    codes = { "A", "H", "V" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 66,
    damage = 140,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
    instant = voltz.instant(2, 20),
    auto_battle = { class = "into_reach", range = "three_ahead" },
}

return { voltz1 = voltz1, voltz2 = voltz2, voltz3 = voltz3 }
