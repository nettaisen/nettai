--!strict
-- TmhwkMnDS (BN5 chip 0x0fd): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:tmhwkmn-ds",
    codes = { "T" },
    element = "wood",
    family = "wood",
    class = "mega",
    rarity = 3,
    mb = 81,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "damage_shown_variable", "library", "variable_damage" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "dark",
}
