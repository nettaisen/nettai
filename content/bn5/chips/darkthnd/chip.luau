--!strict
-- DarkThnd (BN5 chip 0x0c1): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "darkthnd",
    codes = { "M" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 200,
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
}
