--!strict
-- MiniBomb (BN5 chip 0x01d): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "minibomb",
    codes = { "B", "G", "L", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 50,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
