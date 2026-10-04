--!strict
-- A lance (the original's attack object #0x6F, `sub_80D2460`): the shared
-- lance (@exelib/lance/lance), BN6's kind: Lance's three lances
-- (chip.luau).

local collision = require("../../rules/collision")
local common = require("@exelib/lance")
local sparks = require("../../lib/sparks")

local lance = {}

lance.make_kind = common.make_kind
lance.spawn_kind = common.spawn_kind
lance.instant_of = common.instant_of

-- BN6's: the hit (`dword_80D25C8`): one panel, the wood spark, an attack
-- against attacks, hit modifier 0x51.
lance.kind = common.make_kind("lance/lance", {
    sprite = asset.sprite("lance"),
    sound = asset.sound("sword-swing"),
    hits = { [0] = { is = collision.attack, hits = collision.attack, spark = sparks.wood, hit_mod = 0x51 } },
})

-- BN6's lance.
function lance.spawn(owner: Object, x: number, y: number, element: number, z: number, palette: number, damage: number): Object?
    return common.spawn_kind(lance.kind, owner, x, y, element, z, palette, damage)
end

-- BN6's Lance's: Param1 0.
lance.instant = common.instant_of(lance.kind, 0)

return lance
