--!strict
-- Skully3 (BN5 chip 0x047): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "skully3",
    codes = { "G", "N", "U" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 76,
    damage = 140,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
