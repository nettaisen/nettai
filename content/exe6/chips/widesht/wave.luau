--!strict
-- The wide wave (the original's attack object #0x3B, `sub_80CB49C`): the
-- shared wave (@exelib/widesht/wave), EXE6's kind and look. Its variants
-- (`wave.variant { ... }`, the original's kinds by `byte_80CB58C`'s
-- speeds) are its users' (./action): WideSht's ends on its first hit;
-- SuprSpr's goes through, hitting each panel once. See
-- docs/engine/shot-chips.md §10.1.

local collision = require("../../rules/collision")
local common = require("@exelib/widesht/wave")

local wave = {}

-- A wave variant: its speed (16.16 pixels a tick), whether it goes
-- through, and its collision.
wave.variant = common.variant

wave.kind = common.make_kind {
    id = "widesht/wave",
    sprite = asset.sprite("wide-wave"),
    sound = asset.sound("wide-shot"),
    trail_sprite = asset.sprite("rock-cubes"),
    target = collision.hits_navis,
}

-- `sub_80CB686`: a wave of `variant` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element`, the damage word and the hit
-- modifier a piercing variant uses; nil if the attack pool is full.
function wave.fire(owner: Object, x: number, y: number, z: number, variant: WaveVariant, element: number, damage: number, hit_mod: number): Object?
    return common.fire(wave.kind, owner, x, y, z, variant, element, damage, hit_mod)
end

return wave
