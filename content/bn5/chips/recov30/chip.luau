--!strict
-- Recov30 (BN5 chip 0x06c): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:recov30",
    codes = { "C", "H", "L", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 0,
    mb = 16,
    damage = 0,
    hit_param = 30,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
}
