--!strict
-- The DS's Double Attacks (docs/design/exe5ds-map.md §5), the chips whose `double_attack` the party battle's switch
-- fires (rules/double_attacks), in the DS's order (its chips 0x160 on). Each module defines its chip, the attack its
-- navis make (lib/double_attack) and what it draws; the chips' index (chips/init.luau) has them too.

return {
    require("@self/d_hero")["d-hero"],
}
