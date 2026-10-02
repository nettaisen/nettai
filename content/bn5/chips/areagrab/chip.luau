--!strict
-- AreaGrab (BN5 chip 0x075): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "areagrab",
    codes = { "A", "L", "S", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 8,
    damage = 10,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
}
