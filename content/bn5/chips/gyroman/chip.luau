--!strict
-- GyroMan (BN5 chip 0x0e0): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "gyroman",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 32,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
