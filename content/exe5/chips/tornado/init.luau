--!strict
-- Tornado and Static: the tornado series (each chip's own comment above its definition).
-- Their records are written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local attachment = require("../objects/attachment")
local status = require("../rules/status")
local tornado = require("@self/tornado")

-- Tornado's fan (the attachment table's row 0xC) and Static's charged one (row 0x17).
local FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 0, attach_point = 9 }
local CHARGED_FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 1, attach_point = 9 }

-- Tornado's blow (subtype 1): one plain tornado (row 1) two panels ahead, for 0x20 ticks (the record's parameters).
local PLAIN = tornado.variant { palette = 0, rearm = 4, plain = true }
local BLOW = tornado.blow {
    fan = FAN,
    single = { variant = PLAIN, status = nil, ticks = 0x20 },
}

-- Static's blow (subtype 2): static tornadoes (row 2) spreading around the panel two ahead by the navi's kinds of
-- NaviCust bug (0x0800DC14, EXE6's `sub_800FE52` but for its tenth kind, NaviStats +0x63, which EXE5's hasn't), as
-- EXE6's (0x080CADE4, EXE6's `sub_80CA104`: the same panels), the charged fan; EXE5's tornadoes paralyze by level
-- (0x080CAE50: none, 90, 120, 150 ticks; EXE6's none at every level).
local STATIC = tornado.variant { palette = 3, rearm = 4 }
local STATIC_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 1 } },
}
local STATIC_BLOW = tornado.blow {
    fan = CHARGED_FAN,
    spread = {
        level = function(me: Object): number
            return battle.navi(me.alliance).bug_kinds
        end,
        panels = STATIC_SPREAD,
        tornado = { variant = STATIC, status = nil, ticks = 0x20 },
        tornadoes = {
            [0] = { variant = STATIC, status = nil, ticks = 0x20 },
            { variant = STATIC, status = status.paralyze_90, ticks = 0x20 },
            { variant = STATIC, status = status.paralyze_120, ticks = 0x20 },
            { variant = STATIC, status = status.paralyze_150, ticks = 0x20 },
        },
    },
}

-- Tornado (EXE5 chip 0x008): action 0x22, EXE6's blow with EXE5's fan and tornado (chips/tornado/tornado): a plain tornado
-- two panels ahead.
local tornado_chip: Chip = {
    codes = { "D", "L", "R" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 22,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = BLOW,
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

-- Static (EXE5 chip 0x01b): action 0x22, EXE6's blow with EXE5's fan and tornado (chips/tornado/tornado): static tornadoes two
-- panels ahead and around, more the more kinds of NaviCust bug the navi has, paralyzing by that level.
local static: Chip = {
    codes = { "N", "S", "V" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "dark",
    action = STATIC_BLOW,
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

return { tornado = tornado_chip, static = static }
