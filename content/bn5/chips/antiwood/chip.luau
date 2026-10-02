--!strict
-- AntiWood (BN5 chip 0x08f): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:antiwood",
    codes = { "M", "S", "Z", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 200,
    hit_param = 178,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
