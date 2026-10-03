--!strict
-- DethPhnx (BN5 chip 0x13a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:dethphnx",
    codes = { "D" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 93,
    damage = 150,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
}
