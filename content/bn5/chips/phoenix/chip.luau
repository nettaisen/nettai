--!strict
-- Phoenix (BN5 chip 0x139): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:phoenix",
    codes = { "P" },
    element = "fire",
    family = "fire",
    class = "giga",
    rarity = 4,
    mb = 95,
    damage = 150,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi" },
    slot_in_limit = 1,
}
