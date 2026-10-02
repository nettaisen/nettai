--!strict
-- RedWave (BN5 chip 0x0ad): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "redwave",
    codes = { "A", "F", "H" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 48,
    damage = 190,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 3,
}
