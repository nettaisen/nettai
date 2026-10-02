--!strict
-- DrilArm3 (BN5 chip 0x04a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "drilarm3",
    codes = { "I", "P", "Z" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 2,
    mb = 64,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
