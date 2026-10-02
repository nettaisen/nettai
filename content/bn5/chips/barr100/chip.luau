--!strict
-- Barr100 (BN5 chip 0x088): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "barr100",
    codes = { "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 27,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
