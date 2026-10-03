--!strict
-- Navi+20 (BN5 chip 0x096): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:navi-20",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 20,
    hit_param = 0,
    lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { 64, 128 },
    slot_in_limit = 3,
}
