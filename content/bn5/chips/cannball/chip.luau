--!strict
-- CannBall (BN5 chip 0x02c): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "cannball",
    codes = { "C", "T", "Z", "*" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 140,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
