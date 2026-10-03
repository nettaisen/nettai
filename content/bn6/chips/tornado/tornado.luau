--!strict
-- A tornado (the original's attack object #0x31, `sub_80C9F78`): the
-- shared tornado (@common/tornado/tornado), BN6's kind, look and variants:
-- Tornado's and Static's. A plain tornado on a volcano, grass or ice panel
-- takes the panel's variant instead, turns the panel normal and doubles its
-- damage.

local collision = require("../../rules/collision")
local common = require("@common/tornado/tornado")
local sparks = require("../../lib/sparks")

local tornado = {}

-- A tornado variant.
tornado.variant = common.variant
local variant = tornado.variant

-- A tornado kind (`id`) of `look`.
tornado.make_kind = common.make_kind

-- The variants (`byte_80CA064`'s rows 0 to 7): the plain one (1), the ones
-- it takes on grass (0), volcano (4) and ice (6), Static's (2), the one
-- that carries the HP bug (3: bug 0x54, argument 5, `sub_801A4D0`), and
-- two no spawner gives: row 5 and row 7, which re-arms every tick.
tornado.variants = {
    grass = variant { palette = 1, rearm = 4 },
    plain = variant { palette = 0, rearm = 4, plain = true },
    static = variant { palette = 3, rearm = 4 },
    hp_bug = variant { palette = 3, rearm = 4, bug = 0x54 + 5 * 0x100 },
    volcano = variant { palette = 4, rearm = 4 },
    row5 = variant { palette = 2, rearm = 4 },
    ice = variant { palette = 5, rearm = 4 },
    row7 = variant { palette = 0, rearm = 0 },
}

-- BN6's look: its sprite and sound; what a plain tornado becomes on a panel
-- of these types (`byte_80CA25C`), the doubling, what it is (row 0x12,
-- whatever its variant) and reaches, and the element's spark.
local BN6: TornadoLook = {
    sprite = asset.sprite("bn6:tornado"),
    sound = asset.sound("bn6:aqua-surge"),
    panel_variants = {
        volcano = tornado.variants.volcano,
        grass = tornado.variants.grass,
        ice = tornado.variants.ice,
    },
    doubles_on_panel = true,
    is = collision.pushing,
    target = collision.hits_navis,
    spark = sparks.of_element,
}

tornado.kind = common.make_kind("bn6:tornado/tornado", BN6)

-- `sub_80CA0E2`: a tornado for `owner`'s side on (x, y) with the damage
-- word `damage`, of `kind` (BN6's by default).
function tornado.spawn(owner: Object, x: number, y: number, element: number, damage: number, spec: TornadoSpec, kind: Kind?): Object?
    return common.spawn(kind or tornado.kind, owner, x, y, element, damage, spec)
end

return tornado
