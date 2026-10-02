--!strict
-- Recov150 (BN5 chip 0x070): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:recov150",
    codes = { "Q" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 2,
    mb = 48,
    damage = 0,
    hit_param = 50,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
}
