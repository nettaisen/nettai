--!strict
-- MrkCan3 (BN5 chip 0x00e): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "mrkcan3",
    codes = { "E", "R", "Y" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 2,
    mb = 36,
    damage = 130,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
