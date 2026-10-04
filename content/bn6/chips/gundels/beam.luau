--!strict
-- GunDelSol's sun beam (effect object #0x48, `sub_80E5C2C`): the shared
-- beam (@exelib/gundels/beam) with BN6's hum. The light shining on the
-- target column (CrosOver's Django shines it too). Visual only, but it
-- takes an effect slot and a place in the update order. It follows its
-- owner at a fixed offset and ends itself once the owner's slot is
-- cleared. See objects-and-player.md §A.6.
--
-- Its look (`SunBeam`, types.d.luau: the original's first two
-- parameters, a look of `dword_80E5C28` and a palette) is its spawner's.

local common = require("@exelib/gundels/beam")

local beam = {}

-- The looks (`dword_80E5C28`): GunDelSol's, and GunDelEX's wider one.
beam.sprites = {
    plain = asset.sprite("sun-beam"),
    wide = asset.sprite("sun-beam-ex"),
}

-- `sub_80E5D3E`: end the beam (its next update frees it).
beam.stop = common.stop

beam.kind = common.make_kind("gundels/beam", asset.sound("sun-beam"))

-- `sub_80E5D12`: a beam of `look` at `offset` from `owner`, stored in
-- `which`; shown while dimmed too when `while_dimmed`.
function beam.spawn(owner: Object, look: SunBeam, offset: Vec3, which: Slot, while_dimmed: boolean?): Object?
    return common.spawn(beam.kind, owner, look, offset, which, while_dimmed)
end

return beam
