--!strict
-- DarkMetr (BN5 chip 0x0c3): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:darkmetr",
    codes = { "F" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 100,
    hit_param = 132,
    lockout = 20,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
}
