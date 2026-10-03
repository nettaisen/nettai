--!strict
-- LStepSwd (BN5 chip 0x11f): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:lstepswd",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 80,
    damage = 140,
    hit_param = 20,
    lockout = 0,
    flags = { "dimming" },
    slot_in_limit = 1,
}
