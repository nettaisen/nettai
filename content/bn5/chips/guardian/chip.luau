--!strict
-- Guardian (BN5 chip 0x0cb): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:guardian",
    codes = { "O" },
    element = "null",
    family = "summon",
    class = "mega",
    rarity = 3,
    mb = 64,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    slot_in_limit = 1,
}
