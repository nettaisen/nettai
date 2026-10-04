--!strict
-- FullCust (BN5 chip 0x0d1): the custom gauge fills (instant chip effect 5, BN5's 0x080096C4, BN6's `sub_800AF34`,
-- run once by action 0x1C as BN6's); with each side keeping its own gauge (the operation battle), the user's fills too:
-- BN5 sets it full where BN6 adds a third (`sub_802E032`).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

-- A full gauge (of 0x4000): the side's gauge only goes up to it.
local operation_battle = require("../lib/operation_battle")

local FULL = 0x4000

-- 0x080096C4.
local function fill(user: Object, _spec: InstantChipSpec)
    if operation_battle() then
        battle.add_side_gauge(user.alliance, FULL)
        return
    end
    -- sub_801DFA2(0x4000)
    battle.fill_custom_gauge()
end

return define.chip {
    id = "fullcust",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 45,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    instant = fill,
}
