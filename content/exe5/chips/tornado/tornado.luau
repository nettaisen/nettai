--!strict
-- EXE5's tornado (attack object 0x31, 0x080CAC60) and its blow (action 0x22, 0x080ECD46): the shared code
-- (@exelib/tornado: EXE6's) with EXE5's panel variants, collision types and sparks, and EXE5's attachment; its sprite
-- and sound EXE6's numbers. Each chip states its own blow (`tornado.blow`: its fan, its tornadoes' variant, its spread):
-- Tornado's and Static's (./init), DarkTorn's (chips/darktorn).
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

local tornado = {}

-- A tornado variant (a row of 0x080CAD44): its palette, its re-arm ticks, whether it is plain (a panel's variant
-- replaces it) and the bug it carries.
tornado.variant = common.variant
local variant = tornado.variant

-- The variants a plain one takes on grass, lava, magnet and sea (and ice): rows 0, 4, 5 and 6.
local GRASS = variant { palette = 1, rearm = 2 }
local LAVA = variant { palette = 4, rearm = 2 }
local MAGNET = variant { palette = 2, rearm = 2 }
local SEA = variant { palette = 5, rearm = 2 }

tornado.kind = common.make_kind("tornado", {
    sprite = asset.sprite("tornado"),
    sound = asset.sound("aqua-surge"),
    panel_variants = {
        lava = LAVA,
        sea = SEA,
        grass = GRASS,
        magnet = MAGNET,
        ice = SEA,
    },
    doubles_on_panel = false,
    is = collision.collision_20,
    bug_is = collision.collision_4b,
    target = collision.hits_navis,
    spark = sparks.of_element,
})

local LOOK: TornadoBlowLook = { attachment = attachment.kind, tornado = tornado.kind }

-- A chip's blow (action 0x22).
function tornado.blow(spec: TornadoBlow): Action
    return blows.blow(spec, LOOK)
end

return tornado
