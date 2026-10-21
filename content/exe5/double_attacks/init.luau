--!strict
-- The DS's Double Attacks (docs/design/exe5ds-map.md §5), the chips whose `double_attack` the party battle's switch
-- fires (rules/double_attacks), in the DS's order (its chips 0x160 on). Each module defines its chip, the attack its
-- navis make (lib/double_attack) and what it draws; the chips' index (chips/init.luau) has them too.

return {
    require("@self/d_hero")["d-hero"],
    require("@self/m_zan")["m-zan"],
    require("@self/m_air")["m-air"],
    require("@self/d_valcan")["d-valcan"],
    require("@self/c_scope")["c-scope"],
    require("@self/c_bomb")["c-bomb"],
    require("@self/br_rush")["br-rush"],
    require("@self/b_shoot")["b-shoot"],
    require("@self/e_sappou")["e-sappou"],
    require("@self/k_throw")["k-throw"],
    require("@self/k_bomb")["k-bomb"],
    require("@self/g_otoshi")["g-otoshi"],
}
