--!strict
-- ShadeMan (BN5 chip 0x104): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "shademan",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 44,
    damage = 120,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
