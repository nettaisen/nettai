--!strict
-- FireHit's fist (the original's attack object #0x5B, `sub_80CFCF8`) and
-- the chips' effect (instant chip effect 8, `sub_80CFE08`): the shared fist
-- (@exelib/firehit/fist), BN6's kind and look. The chips are ./chips
-- (FireHit1-3 are the ingredients of FlmHook's Program Advances).

local collision = require("../../rules/collision")
local common = require("@exelib/firehit/fist")
local sparks = require("../../lib/sparks")

local fist = {}

-- BN6's look: its sprite and sound, the strike's spark and collision types
-- (`dword_80CFEC0`'s bytes 1 to 3).
local BN6: FireHitFistLook = {
    sprite = asset.sprite("bn6:fire-hit"),
    sound = asset.sound("bn6:fire-hit"),
    spark = sparks.fire,
    target = collision.hits_navis,
    self_type = collision.attack,
}

fist.kind = common.make_kind("bn6:firehit/fist", BN6)

-- `sub_80CFDF2` and `sub_801155A`: a fist of `owner`'s on (x, y); of
-- `kind`, BN6's by default.
function fist.spawn(owner: Object, x: number, y: number, element: number, z: number, hit_mod: number, damage: number, kind: Kind?): Object?
    return common.spawn(kind or fist.kind, owner, x, y, element, z, hit_mod, damage)
end

-- `sub_80CFE08`, instant chip effect 8: an `instant` hook whose fists (of
-- `kind`, BN6's by default) hit with modifier `hit_mod`.
function fist.instant(hit_mod: number, kind: Kind?): (user: Object, spec: InstantChipSpec) -> ()
    return common.instant(hit_mod, kind or fist.kind)
end

return fist
