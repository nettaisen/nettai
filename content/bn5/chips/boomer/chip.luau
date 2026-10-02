--!strict
-- Boomer (BN5 chip 0x006): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "boomer",
    codes = { "A", "L", "V", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 16,
    damage = 60,
    hit_param = 10,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
