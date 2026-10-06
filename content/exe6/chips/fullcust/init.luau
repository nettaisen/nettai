--!strict
-- FullCust: the custom gauge fills (instant chip effect 5, `sub_800AF34`,
-- run once by action 0x1C); with each side keeping its own gauge (battle
-- flag 0x40), the user's gains a third instead.

-- What a side's own gauge gains (of 0x4000).
local chip_gate_battle = require("../lib/chip_gate_battle")

local SIDE_GAUGE = 0x1555

-- `sub_800AF34`.
local function fill(user: Object, _spec: InstantChipSpec)
    if chip_gate_battle() then
        -- sub_802E032
        battle.add_side_gauge(user.alliance, SIDE_GAUGE)
        return
    end
    -- sub_801DFA2(0x4000)
    battle.fill_custom_gauge()
end

local chip: Chip = {
    codes = { "*" },
    element = "null", family = "null", class = "standard",
    rarity = 3, mb = 50, damage = 0, hit_param = 0, lockout = 20,
    flags = { "standard_library", "library" },
    extra_flags = { "free_slot_in" },
    slot_in_limit = 1,
    instant = fill,
}

return { fullcust = chip }
