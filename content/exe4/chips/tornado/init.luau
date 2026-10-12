--!strict
-- Tornado and Static (EXE4 chips 0x008 and 0x01B; their records at 0x080197EC + n * 0x2C): the fan's action 0x28
-- (./fan, its variants 1 and 2): the fan (attachment row 0x0F, Static's 0x26: 0c-16 in palette 0 or 1) and each
-- chip's blow of tornadoes (./tornado).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local fan = require("@self/fan")
local navicust = require("../rules/navicust")
local statuses = require("../rules/status")
local tornado = require("@self/tornado")

local FAN = asset.sprite("hand-fan")
local TICKS = 0x20

-- Tornado's blow (0x080D3130): one tornado (its parameters: the first 1, palette 0; no status; 0x20 ticks).
local PLAIN = tornado.look { palette = 0, status = nil, ticks = TICKS }
local function single(me: Object, x: number, y: number, element: number, damage: number)
    tornado.spawn(me, x, y, element, damage, PLAIN)
end

-- Static's blow (0x080D3152): static tornadoes (palette 3) spreading by the navi's kinds of NaviCust bug (rules/
-- navicust's `bug_kinds`, 0x08043014; 3 at most), each level more panels (0x080D31AC) and a longer paralysis
-- (0x080D31BC: none, 90, 120, 150 ticks).
local STATIC = {
    tornado.look { palette = 3, status = nil, ticks = TICKS },
    tornado.look { palette = 3, status = statuses.paralyze_90, ticks = TICKS },
    tornado.look { palette = 3, status = statuses.paralyze_120, ticks = TICKS },
    tornado.look { palette = 3, status = statuses.paralyze_150, ticks = TICKS },
}
local SPREAD = {
    { { 0, 0 } },
    { { 0, 0 }, { 1, 0 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 } },
    { { 0, 0 }, { 1, 0 }, { 0, -1 }, { 0, 1 }, { 1, -1 }, { 1, 1 } },
}
local function spread(me: Object, x: number, y: number, element: number, damage: number)
    local level = math.min(navicust.bug_kinds(me.alliance), 3) + 1
    local look = STATIC[level]
    for _, d in SPREAD[level] do
        tornado.spawn(me, x + d[1] * me.facing, y + d[2], element, damage, look)
    end
end

local tornado_chip: Chip = {
    action = fan.action("tornado/action", attachment.look { sprite = FAN, palette = 0 }, single),
    codes = { "E", "L", "T" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 22,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

local static: Chip = {
    action = fan.action("static/action", attachment.look { sprite = FAN, palette = 1 }, spread),
    codes = { "B", "G", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    megaman = "dark",
    auto_battle = { class = "into_reach", range = "two_ahead" },
}

return { tornado = tornado_chip, static = static }
