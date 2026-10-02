--!strict
-- BigHook (BN5 chip 0x134): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bighook",
    codes = { "G" },
    element = "null",
    family = "break",
    class = "giga",
    rarity = 4,
    mb = 92,
    damage = 240,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
    megaman = "light",
}
