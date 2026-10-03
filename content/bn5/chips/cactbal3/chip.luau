--!strict
-- CactBal3 (BN5 chip 0x0b1): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:cactbal3",
    codes = { "L", "M", "N" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 50,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
