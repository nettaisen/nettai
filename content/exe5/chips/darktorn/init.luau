--!strict
-- DarkTorn (EXE5 chip 0x0c5): a dark chip: action 0x22's third form, HP-bug tornadoes spreading back by the side's
-- statistic 3 (chips/tornado/tornado). Its use costs custom-screen damage (rules/light_dark).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local attachment = require("../objects/attachment")
local tornado = require("./tornado/tornado")
local dark_costs = require("../lib/dark_costs")

-- Its blow (subtype 3, EXE6's action's third form): HP-bug tornadoes (row 3: bug 0x54, argument 5) spreading back
-- toward the navi by its side's statistic 3 (0x080CAE7E, EXE6's `sub_80CA19E`: the same tables, `off_80CA1F8`), the
-- charged fan (the attachment table's row 0x17) one palette on.
local HP_BUG = tornado.variant { palette = 3, rearm = 4, bug = 0x54 + 5 * 0x100 }
local SIDE_STAT = 3
local BACK_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { -1, 0 } },
    { { 0, 0 }, { -1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { -1, 0 }, { 0, -1 }, { 0, 1 }, { 1, 0 } },
}
local BLOW = tornado.blow {
    fan = attachment.look { sprite = asset.sprite("hand-fan"), palette = 1, attach_point = 9 },
    fan_palette = 1,
    spread = {
        level = function(me: Object): number
            return battle.side_stat(me.alliance, SIDE_STAT)
        end,
        panels = BACK_SPREAD,
        tornado = { variant = HP_BUG, status = nil, ticks = 0x20 },
    },
}

local chip: Chip = {
    codes = { "T" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 50,
    hit_param = 132,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = BLOW,
    -- What its use costs its user (rules/light_dark).
    dark_cost = dark_costs.custom_damage,
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

return { darktorn = chip }
