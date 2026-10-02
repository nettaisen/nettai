--!strict
-- HotBody1 (BN5 chip 0x09a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "hotbody1",
    codes = { "A", "F", "P" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 28,
    damage = 110,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
