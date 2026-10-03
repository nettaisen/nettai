--!strict
-- BN5's tornado (attack object 0x31, 0x080CAC60) and its blow (action 0x22, 0x080ECD46): BN6's code
-- (@bn6/chips/tornado) with BN5's variants, panels, collision types and sparks, and BN5's fan.
--
-- - The variants (0x080CAD44: palette, re-arm ticks) are BN6's rows with re-arms of 2 for rows 0, 4, 5 and 6 (BN6's
--   4).
-- - A plain tornado takes a panel's variant on lava (row 4), sea (6), grass (0), metal (5) and ice (6) (0x080CAF3C),
--   turns the panel Normal, and doesn't double its damage (BN6 does).
-- - It is BN5's row 0x20 (BN6's 0x12), the HP-bug variant's row 0x4B, reaching row 5; its spark BN5's of its element.
-- - The fan is BN5's attachment rows 0xC and 0x17 (BN6's 0x21 and 0x22: the same look in BN5's sprite).

local attachment = require("@bn6/objects/attachment/attachment")
local bn6_tornado = require("@bn6/chips/tornado/tornado")
local blows = require("@bn6/chips/tornado/chips")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local tornado = {}

local variant = bn6_tornado.variant
tornado.variants = {
    grass = variant { palette = 1, rearm = 2 },
    plain = variant { palette = 0, rearm = 4, plain = true },
    static = variant { palette = 3, rearm = 4 },
    hp_bug = variant { palette = 3, rearm = 4, bug = 0x54 + 5 * 0x100 },
    lava = variant { palette = 4, rearm = 2 },
    metal = variant { palette = 2, rearm = 2 },
    sea = variant { palette = 5, rearm = 2 },
    row7 = variant { palette = 0, rearm = 0 },
}

tornado.kind = bn6_tornado.make_kind("bn5:tornado", {
    panel_variants = {
        lava = tornado.variants.lava,
        sea = tornado.variants.sea,
        grass = tornado.variants.grass,
        metal = tornado.variants.metal,
        ice = tornado.variants.sea,
    },
    doubles_on_panel = false,
    is = collision.collision_20,
    bug_is = collision.collision_4b,
    target = collision.hits_navis,
    spark = sparks.of_element,
})

local FAN = attachment.look { sprite = asset.sprite("bn5:hand-fan"), palette = 0, attach_point = 9 }

-- Tornado's blow (subtype 1): one plain tornado two panels ahead, for 0x20 ticks (the record's parameters).
tornado.single = blows.blow {
    id = "bn5:tornado/action",
    fan = FAN,
    tornado_kind = tornado.kind,
    single = { variant = tornado.variants.plain, status = nil, ticks = 0x20 },
}

return tornado
