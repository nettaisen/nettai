--!strict
-- JustcOne (BN5 chip 0x0d8): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:justcone",
    codes = { "J" },
    element = "null",
    family = "break",
    class = "mega",
    rarity = 4,
    mb = 90,
    damage = 280,
    hit_param = 178,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
