--!strict
-- WideSht3 (BN5 chip 0x00b): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "widesht3",
    codes = { "S", "T", "U" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
