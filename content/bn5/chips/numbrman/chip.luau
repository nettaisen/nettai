--!strict
-- NumbrMan (BN5 chip 0x0f2): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "numbrman",
    codes = { "N" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 33,
    damage = 30,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
}
