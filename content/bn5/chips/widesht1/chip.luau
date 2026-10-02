--!strict
-- WideSht1 (BN5 chip 0x009): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:widesht1",
    codes = { "L", "M", "N" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 0,
    mb = 10,
    damage = 70,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
