--!strict
-- BatCan2 (BN5 chip 0x13c): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:batcan2",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage" },
    slot_in_limit = 1,
}
