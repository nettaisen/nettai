--!strict
-- Magnum (BN5 chip 0x05e): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "magnum",
    codes = { "E", "O", "V" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 130,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
