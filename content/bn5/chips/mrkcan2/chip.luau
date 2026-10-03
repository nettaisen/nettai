--!strict
-- MrkCan2 (BN5 chip 0x00d): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:mrkcan2",
    codes = { "D", "F", "Z" },
    element = "null",
    family = "cursor",
    class = "standard",
    rarity = 1,
    mb = 27,
    damage = 100,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
