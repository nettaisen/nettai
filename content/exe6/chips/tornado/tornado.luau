--!strict
-- A tornado (the original's attack object #0x31, `sub_80C9F78`): the
-- shared tornado (@exelib/tornado/tornado), EXE6's kind and look; each
-- chip states its variant (`tornado.variant`: Tornado's and Static's,
-- chips/tornado/init.luau). A plain tornado on a volcano, grass or ice panel
-- takes the panel's variant instead, turns the panel normal and doubles its
-- damage.

local collision = require("../../rules/collision")
local common = require("@exelib/tornado")
local sparks = require("../../lib/sparks")

local tornado = {}

-- A tornado variant.
tornado.variant = common.variant
local variant = tornado.variant

-- A tornado kind (`id`) of `look`.
tornado.make_kind = common.make_kind

-- The variants a plain one takes on grass, volcano and ice
-- (`byte_80CA064`'s rows 0, 4 and 6). (The chips' are their own: Tornado's
-- plain one, row 1, and Static's, row 2. No spawner gives rows 3, 5 and 7.)
local GRASS = variant { palette = 1, rearm = 4 }
local VOLCANO = variant { palette = 4, rearm = 4 }
local ICE = variant { palette = 5, rearm = 4 }

-- EXE6's look: its sprite and sound; what a plain tornado becomes on a panel
-- of these types (`byte_80CA25C`), the doubling, what it is (row 0x12,
-- whatever its variant) and reaches, and the element's spark.
local EXE6: TornadoLook = {
    sprite = asset.sprite("tornado"),
    sound = asset.sound("aqua-surge"),
    panel_variants = {
        volcano = VOLCANO,
        grass = GRASS,
        ice = ICE,
    },
    doubles_on_panel = true,
    is = collision.pushing,
    target = collision.hits_navis,
    spark = sparks.of_element,
}

tornado.kind = common.make_kind("tornado/tornado", EXE6)

-- `sub_80CA0E2`: a tornado for `owner`'s side on (x, y) with the damage
-- word `damage`, of `kind` (EXE6's by default).
function tornado.spawn(owner: Object, x: number, y: number, element: number, damage: number, spec: TornadoSpec, kind: Kind?): Object?
    return common.spawn(kind or tornado.kind, owner, x, y, element, damage, spec)
end

return tornado
