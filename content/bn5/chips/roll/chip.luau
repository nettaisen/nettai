--!strict
-- Roll (BN5 chip 0x0dd): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "roll",
    codes = { "R" },
    element = "null",
    family = "recovery",
    class = "mega",
    rarity = 2,
    mb = 20,
    damage = 20,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
