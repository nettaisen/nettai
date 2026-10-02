--!strict
-- Recov10 (BN5 chip 0x06b): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "recov10",
    codes = { "A", "C", "L", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 0,
    hit_param = 30,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
}
