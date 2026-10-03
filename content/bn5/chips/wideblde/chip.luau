--!strict
-- WideBlde (BN5 chip 0x033): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:wideblde",
    codes = { "A", "L", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
