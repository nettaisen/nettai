--!strict
-- Tornado and Static (the original's action 0x2F, `sub_80ED454`; the
-- shared blow, @exelib/tornado/blow, with BN6's attachment and tornado):
-- the navi
-- holds out a fan (an attachment) and, ten ticks in, raises tornadoes
-- (tornado.luau) two panels ahead; the fan turns while they blow. Tornado
-- raises one on that panel; Static raises them on more panels the more
-- kinds of NaviCust bug the navi has (none: that panel; up to three: also
-- the one behind it, the two beside it, and the two beside that one), a
-- static tornado each. Another game's chips take the blow (`chips.blow`)
-- with their fan and tornado kind. The action's third form (its subtype 3, which no
-- chip has) does the same by a side statistic (`sub_800AB3A`, counter 3),
-- spreading back toward the navi, with HP-bug tornadoes, and a fan of
-- another palette. 42 ticks, then 21 to put the fan away.

local attachment = require("../../objects/attachment/attachment")
local common = require("@exelib/tornado/blow")
local lockon = require("../../rules/lockon")
local tornado = require("./tornado")

-- The fan (the original's attachment rows 0x21 and 0x22).
local FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 0, attach_point = 9 }
local CHARGED_FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 1, attach_point = 9 }

local LOOK: TornadoBlowLook = { attachment = attachment.kind, tornado = tornado.kind }

-- A chip's blow (@exelib/tornado/blow), with BN6's attachment and tornado.
local function blow(spec: TornadoBlow): Action
    return common.blow(spec, LOOK)
end

local STATIC_SPREAD, BACK_SPREAD = common.STATIC_SPREAD, common.BACK_SPREAD

local chips = {
    define.chip {
        id = "tornado",
        codes = { "L", "R", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 16, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 53, index = 53, sort = 442 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.firebrn1 },
        action = blow {
            fan = FAN,
            single = { variant = tornado.variants.plain, status = nil, ticks = 0x20 },
        },
    },
    define.chip {
        id = "static",
        codes = { "G", "S", "V" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 30, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        library = { number = 54, index = 54, sort = 420 },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.firebrn1 },
        action = blow {
            fan = CHARGED_FAN,
            spread = {
                by = "bug_kinds",
                panels = STATIC_SPREAD,
                tornado = { variant = tornado.variants.static, status = nil, ticks = 0x20 },
            },
        },
    },
}

-- The action's third form, which no chip has (the original's subtype 3,
-- with `byte_80CA208`'s tornadoes).
chips.back_spread = blow {
    id = "tornado/back-spread",
    fan = CHARGED_FAN,
    fan_palette = 1,
    spread = {
        by = "side_stat",
        panels = BACK_SPREAD,
        tornado = { variant = tornado.variants.hp_bug, status = nil, ticks = 0x20 },
    },
}

chips.STATE = common.STATE
-- The blow and the spreads.
chips.blow = blow
chips.STATIC_SPREAD, chips.BACK_SPREAD = STATIC_SPREAD, BACK_SPREAD

return chips
