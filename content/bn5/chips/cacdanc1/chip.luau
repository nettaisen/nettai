--!strict
-- CacDanc1 (BN5 chip 0x14f): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "cacdanc1",
    codes = {},
    element = "wood",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 30,
    hit_param = 5,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
}
