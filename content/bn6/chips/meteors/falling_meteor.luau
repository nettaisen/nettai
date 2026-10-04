--!strict
-- A falling meteor (the original's attack object #0x56, `sub_80CF3BE`): the
-- shared meteor (@exelib/meteors/falling_meteor), BN6's kind and look.
-- Meteors (./chip) drops thirty; the instant chips' meteor shower
-- (lib/instant/meteor_shower) uses it too.

local collision = require("../../rules/collision")
local common = require("@exelib/meteors/falling_meteor")
local effects = require("../../lib/effects")
local marker = require("./marker")
local sparks = require("../../lib/sparks")

local falling_meteor = {}

falling_meteor.make_kind = common.make_kind
falling_meteor.spawn_kind = common.spawn_kind

-- BN6's rows 0 to 2, alike: thrown, hitting navis, hit modifier 3; a
-- shake of 1; the panel left as it is.
local BN6_ROW: MeteorRow = { is = collision.thrown, hits = collision.hits_navis, hit_mod = 3, shake = 1, panel = "none" }

falling_meteor.kind = common.make_kind("bn6:meteors/falling-meteor", {
    sprite = asset.sprite("bn6:meteor"),
    sounds = { fall = asset.sound("bn6:justice-one"), land = asset.sound("bn6:hit-bomb-1") },
    rows = { [0] = BN6_ROW, [1] = BN6_ROW, [2] = BN6_ROW },
    spark = sparks.of_element,
    burst = effects.burst,
    marker = marker.kind,
})

-- BN6's meteor of row `row`.
function falling_meteor.spawn(
    row: number,
    owner: Object,
    x: number,
    y: number,
    element: number,
    damage: number,
    while_dimmed: boolean,
    pos: Vec3?
): Object?
    return common.spawn_kind(falling_meteor.kind, row, owner, x, y, element, damage, while_dimmed, pos)
end

return falling_meteor
