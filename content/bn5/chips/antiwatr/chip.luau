--!strict
-- AntiWatr (BN5 chip 0x08d): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "antiwatr",
    codes = { "D", "G", "Q", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 2,
    mb = 54,
    damage = 200,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
