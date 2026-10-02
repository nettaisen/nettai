--!strict
-- BassAnly (BN5 chip 0x132): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bassanly",
    codes = { "F" },
    element = "null",
    family = "null",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 160,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    extra_flags = { 8, 16 },
    slot_in_limit = 1,
}
