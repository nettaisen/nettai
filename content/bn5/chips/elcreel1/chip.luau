--!strict
-- ElcReel1 (BN5 chip 0x0a3): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "elcreel1",
    codes = { "C", "V", "Z" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 80,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
