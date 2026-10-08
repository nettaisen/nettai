--!strict
-- What a dark chip's use costs its user (rules/light_dark, 0x0802C934: the chip's entry, a NaviCust bug of the battle
-- by a routine of 0x0802C974's), each dark chip's own `dark_cost` (a `DarkCost`). Several chips' entries share a
-- routine, which this gives (`dark_costs.custom_damage` ...); a chip with a routine of its own makes it
-- (`dark_costs.cost`).

local dark_costs = {}

function dark_costs.cost(apply: (stats: Navi) -> ()): DarkCost
    return new.record("exe5-dark-cost", { apply = apply })
end

-- The custom screen's damage, 100 more (DarkCirc's, DarkTorn's).
dark_costs.custom_damage = dark_costs.cost(function(stats: Navi)
    stats.custom_damage = bit32.band(stats.custom_damage + 100, 0xFFFF)
end)

-- Four buster blanks (DrkSword's, DarkWide's).
dark_costs.buster_blanks = dark_costs.cost(function(stats: Navi)
    stats.buster_blanks = 4
end)

-- The step bug (DarkInvs's, DarkPlus's, DrkSonic's).
dark_costs.step_bug = dark_costs.cost(function(stats: Navi)
    stats.step_bug = 1
end)

-- The custom screen's drain, 2 more, at most 7 (DarkThnd's, DarkMetr's).
dark_costs.custom_drain = dark_costs.cost(function(stats: Navi)
    stats.custom_drain = math.min(stats.custom_drain + 2, 7)
end)

return dark_costs
