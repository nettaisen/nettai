--!strict
-- HotBody3 (BN5 chip 0x09c): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:hotbody3",
    codes = { "J", "M", "S" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 52,
    damage = 150,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
