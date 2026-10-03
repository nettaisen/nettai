--!strict
-- Lance (BN5 chip 0x051): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:lance",
    codes = { "C", "L", "R", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 42,
    damage = 130,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
