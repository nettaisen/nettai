--!strict
-- BlakWing (BN5 chip 0x0d6): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:blakwing",
    codes = { "W" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 58,
    damage = 20,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
    megaman = "dark",
}
