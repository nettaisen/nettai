--!strict
-- WoodNos1 (BN5 chip 0x0b2): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "woodnos1",
    codes = { "F", "R", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 0,
    mb = 27,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
