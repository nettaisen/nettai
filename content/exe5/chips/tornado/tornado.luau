--!strict
-- EXE5's tornado (attack object 0x31, 0x080CAC60) and its blow (action 0x22, 0x080ECD46): the shared code
-- (@exelib/tornado: EXE6's) with EXE5's variants, panels, collision types and sparks, EXE5's fan and attachment; its
-- sprite and sound EXE6's numbers.
--
-- - The variants (0x080CAD44: palette, re-arm ticks) are EXE6's rows with re-arms of 2 for rows 0, 4, 5 and 6 (EXE6's
--   4).
-- - A plain tornado takes a panel's variant on lava (row 4), sea (6), grass (0), magnet (5) and ice (6) (0x080CAF3C),
--   turns the panel Normal, and doesn't double its damage (EXE6 does).
-- - It is EXE5's row 0x20 (EXE6's 0x12), the HP-bug variant's row 0x4B, reaching row 5; its spark EXE5's of its element.
-- - The fan is EXE5's attachment rows 0xC and 0x17 (EXE6's 0x21 and 0x22: the same look in EXE5's sprite).

local attachment = require("../../objects/attachment")
local blows = require("@exelib/tornado/blow")
local common = require("@exelib/tornado")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local status = require("../../rules/status")

local tornado = {}

local variant = common.variant
tornado.variants = {
    grass = variant { palette = 1, rearm = 2 },
    plain = variant { palette = 0, rearm = 4, plain = true },
    static = variant { palette = 3, rearm = 4 },
    hp_bug = variant { palette = 3, rearm = 4, bug = 0x54 + 5 * 0x100 },
    lava = variant { palette = 4, rearm = 2 },
    magnet = variant { palette = 2, rearm = 2 },
    sea = variant { palette = 5, rearm = 2 },
    row7 = variant { palette = 0, rearm = 0 },
}

tornado.kind = common.make_kind("tornado", {
    sprite = asset.sprite("tornado"),
    sound = asset.sound("aqua-surge"),
    panel_variants = {
        lava = tornado.variants.lava,
        sea = tornado.variants.sea,
        grass = tornado.variants.grass,
        magnet = tornado.variants.magnet,
        ice = tornado.variants.sea,
    },
    doubles_on_panel = false,
    is = collision.collision_20,
    bug_is = collision.collision_4b,
    target = collision.hits_navis,
    spark = sparks.of_element,
})

local FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 0, attach_point = 9 }
local CHARGED_FAN = attachment.look { sprite = asset.sprite("hand-fan"), palette = 1, attach_point = 9 }

local LOOK: TornadoBlowLook = { attachment = attachment.kind, tornado = tornado.kind }

-- Tornado's blow (subtype 1): one plain tornado two panels ahead, for 0x20 ticks (the record's parameters).
tornado.single = blows.blow({
    fan = FAN,
    single = { variant = tornado.variants.plain, status = nil, ticks = 0x20 },
}, LOOK)

-- Static's blow (subtype 2): static tornadoes spreading around the panel two ahead by the navi's kinds of NaviCust
-- bug (0x0800DC14, EXE6's `sub_800FE52` but for its tenth kind, NaviStats +0x63, which EXE5's hasn't), as EXE6's
-- (0x080CADE4, EXE6's `sub_80CA104`: the same panels), the charged fan; EXE5's tornadoes paralyze by level
-- (0x080CAE50: none, 90, 120, 150 ticks; EXE6's none at every level).
tornado.static = blows.blow({
    fan = CHARGED_FAN,
    spread = {
        by = "bug_kinds",
        panels = blows.STATIC_SPREAD,
        tornado = { variant = tornado.variants.static, status = nil, ticks = 0x20 },
        tornadoes = {
            [0] = { variant = tornado.variants.static, status = nil, ticks = 0x20 },
            { variant = tornado.variants.static, status = status.paralyze_90, ticks = 0x20 },
            { variant = tornado.variants.static, status = status.paralyze_120, ticks = 0x20 },
            { variant = tornado.variants.static, status = status.paralyze_150, ticks = 0x20 },
        },
    },
}, LOOK)

-- DarkTorn's blow (subtype 3, EXE6's action's third form): HP-bug tornadoes spreading back toward the navi by its
-- side's statistic 3 (0x080CAE7E, EXE6's `sub_80CA19E`: the same tables), the charged fan one palette on.
tornado.back_spread = blows.blow({
    fan = CHARGED_FAN,
    fan_palette = 1,
    spread = {
        by = "side_stat",
        panels = blows.BACK_SPREAD,
        tornado = { variant = tornado.variants.hp_bug, status = nil, ticks = 0x20 },
    },
}, LOOK)

return tornado
