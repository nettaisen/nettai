--!strict
-- Django (BN5 chip 0x116): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:django",
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 2,
    mb = 40,
    damage = 120,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
