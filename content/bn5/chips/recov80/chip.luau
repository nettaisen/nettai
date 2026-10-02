--!strict
-- Recov80 (BN5 chip 0x06e): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:recov80",
    codes = { "B", "K", "O", "*" },
    element = "null",
    family = "recovery",
    class = "standard",
    rarity = 1,
    mb = 32,
    damage = 0,
    hit_param = 40,
    lockout = 30,
    flags = { "standard_library", "library" },
    slot_in_limit = 1,
}
