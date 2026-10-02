--!strict
-- SeaSeed (BN5 chip 0x02f): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:seaseed",
    codes = { "D", "J", "M", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
}
