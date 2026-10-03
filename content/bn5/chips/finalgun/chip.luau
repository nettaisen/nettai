--!strict
-- FinalGun (BN5 chip 0x122): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:finalgun",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 80,
    damage = 50,
    hit_param = 10,
    lockout = 20,
    flags = { "has_damage" },
    extra_flags = { 16, 32 },
    slot_in_limit = 1,
}
