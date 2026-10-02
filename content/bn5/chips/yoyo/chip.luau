--!strict
-- YoYo (BN5 chip 0x052): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "yoyo",
    codes = { "D", "E", "F" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 32,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
