--!strict
-- FullCust: instant chip effect 5 (`sub_800AF34`, `off_80EC3F0[5]`), run
-- once by action 0x1C. The custom gauge fills; with each side keeping its
-- own gauge (battle flag 0x40), the user's gains a third instead.

local full_cust = {}

-- What a side's own gauge gains (of 0x4000).
local SIDE_GAUGE = 0x1555

-- `sub_800AF34`.
function full_cust.instant_chip(user: Object, _spec: InstantChipSpec)
    if battle.per_player_gauges() then
        -- sub_802E032
        battle.add_side_gauge(user.alliance, SIDE_GAUGE)
        return
    end
    -- sub_801DFA2(0x4000)
    battle.fill_custom_gauge()
end

return full_cust
