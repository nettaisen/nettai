--!strict
-- MudWave (BN5 chip 0x0ae): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "mudwave",
    codes = { "D", "N", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 3,
    mb = 54,
    damage = 170,
    hit_param = 30,
    lockout = 20,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 3,
}
