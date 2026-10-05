--!strict
-- The instruments (the original's attack object #0x78, `sub_80D4230`): the
-- shared instrument (@exelib/instruments/instrument), EXE6's kind and look.

local absorbed_obstacle = require("../../objects/absorbed-obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/instruments/instrument")
local effects = require("../effects")

local instrument = {}

instrument.instrument = common.instrument
instrument.make_kind = common.make_kind
instrument.spawn_kind = common.spawn_kind

local EXE6_SPRITE = asset.sprite("instrument")

instrument.kind = common.make_kind("instrument", {
    sprite = EXE6_SPRITE,
    hp = 60,
    sounds = { appear = asset.sound("appear"), broken = asset.sound("hit-bomb-1") },
    -- Another body of its side, reached by the other side.
    body = { is = collision.own_body, hits = collision.hit_by_other_side },
    -- How it looks absorbed (the original's obstacle kind's sprite: its
    -- own).
    absorbed = absorbed_obstacle.look { sprite = EXE6_SPRITE },
    explosion = effects.explosion,
    puff = effects.puff,
})

-- EXE6's instrument.
function instrument.spawn(owner: Object, x: number, y: number, i: Instrument, damage: number): Object?
    return common.spawn_kind(instrument.kind, owner, x, y, i, damage)
end

return instrument
