--!strict
-- A drill (the original's attack object #0x71, `sub_80D2A94`): the shared
-- drill (@exelib/drilarm/drill), BN6's kind: DrilArm's (./chip), which
-- GroundCross's charged shot (navis/megaman/forms/groundcross/drill) and
-- MstrCros's GroundCross (chips/mstrcros/navi) use too.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/drilarm/drill")
local sparks = require("../../lib/sparks")

local drill = {}

-- The drill arm the navi wears while it drills (the original's attachment
-- row 0x20), in its drilling animation (1) or drawn back (0).
drill.arm = attachment.look { sprite = asset.sprite("bn6:drill-arm"), palette = 0, attach_point = 0x13 }
drill.ARM_DRILLING = 1

local bn6_kind, bn6_spawn = common.make_kind("bn6:drilarm/drill", {
    spark = sparks.breaking,
    first = collision.breaking,
    second = collision.second_drill,
    hits = collision.hits_navis,
    param3 = "light",
    keeps_element = true,
})
drill.kind = bn6_kind

-- `sub_80D2B8E` from its user: the first drill for `owner`, held in its
-- first related object (a navi's attack), or in its state's `child` (a
-- navi chip's navi, whose kind has that field).
function drill.spawn(
    owner: Object,
    element: number,
    z: number,
    damage: number,
    spec: { ticks: number, light: boolean? },
    slot: ("related" | "child")?
): Object?
    return bn6_spawn(owner, element, z, damage, spec, slot)
end

return drill
