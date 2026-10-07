--!strict
-- Tornado and Static (the original's action 0x2F, `sub_80ED454`; the
-- shared blow, @exelib/tornado/blow, with EXE6's attachment and tornado):
-- the navi
-- holds out a fan (an attachment) and, ten ticks in, raises tornadoes
-- (tornado.luau) two panels ahead; the fan turns while they blow. Tornado
-- raises one on that panel, a plain tornado (`byte_80CA064`'s row 1); Static
-- raises them on more panels the more kinds of NaviCust bug the navi has
-- (none: that panel; up to three: also the one behind it, the two beside
-- it, and the two beside that one), a static tornado each (row 2). 42
-- ticks, then 21 to put the fan away.

local attachment = require("../objects/attachment")
local common = require("@exelib/tornado/blow")
local lockon = require("../rules/lockon")
local tornado = require("@self/tornado")

-- The fan (the original's attachment rows 0x21 and 0x22).
local FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 0, attach_point = 9 }
local CHARGED_FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 1, attach_point = 9 }

local LOOK: TornadoBlowLook = { attachment = attachment.kind, tornado = tornado.kind }

-- A chip's blow (@exelib/tornado/blow), with EXE6's attachment and tornado.
local function blow(spec: TornadoBlow): Action
    return common.blow(spec, LOOK)
end

-- Tornado's tornado (row 1): a plain one, which a panel's variant replaces.
local PLAIN = tornado.variant { palette = 0, rearm = 4, plain = true }
-- Static's (row 2).
local STATIC = tornado.variant { palette = 3, rearm = 4 }
-- Static's spread (`off_80CA160`), by its level, the navi's kinds of NaviCust bug.
local STATIC_SPREAD = {
    [0] = { { 0, 0 } },
    { { 0, 0 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 1 } },
}
local function bug_kinds(me: Object): number
    return battle.navi(me.alliance).bug_kinds
end

local chips: { [string]: Chip } = {
    tornado = {
        codes = { "L", "R", "T" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 16, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.firebrn1 },
        action = blow {
            fan = FAN,
            single = { variant = PLAIN, status = nil, ticks = 0x20 },
        },
    },
    static = {
        codes = { "G", "S", "V" },
        element = "null", family = "wind", class = "standard",
        rarity = 2, mb = 30, damage = 20, hit_param = 4, lockout = 0,
        flags = { "has_damage", "standard_library", "library" },
        slot_in_limit = 3,
        -- The Beast rush, and its lock-on mode (rules/lockon).
        beast = { lockon = lockon.firebrn1 },
        action = blow {
            fan = CHARGED_FAN,
            spread = {
                level = bug_kinds,
                panels = STATIC_SPREAD,
                tornado = { variant = STATIC, status = nil, ticks = 0x20 },
            },
        },
    },
}

return chips
