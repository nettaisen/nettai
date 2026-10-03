--!strict
-- RainyDay (BN5 chip 0x07e): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:rainyday",
    codes = { "B", "G", "L" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 47,
    damage = 40,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 3,
}
