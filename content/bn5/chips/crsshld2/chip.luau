--!strict
-- CrsShld2 (BN5 chip 0x0aa): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "crsshld2",
    codes = { "C", "M", "R" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 160,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
