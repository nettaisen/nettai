--!strict
-- DeltaRay, the navi of EXE5's DeltaRay chip (EXE5's actor object #0x3D, 0x080C2018): the shared DeltaRay
-- (@exelib/deltaray/navi), EXE5's kind. EXE5's code is EXE6's but for its constants (ProtoMan's sprite 08-01, its
-- sounds, its mark (effect 0x35) and slash (effect 0x33), his sword's animation 1, his finish's animation 0x11, the
-- follow effect's look 4 for his burst), its first candidate panel (the column past the enemy navi, 0x080C1C0C),
-- no white flashes at the strike or the burst, and, in the operation battle, its aim at the enemy navi his
-- side tracks (0x080C1E68). His palette is navi 1's count by the level (0x0800DA8A: 0 for the chip's level 0).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/deltaray/navi")
local follow_effect = require("./follow_effect")

return common.make {
    id = "deltaray/navi",
    sprite = asset.sprite("protoman"),
    sounds = {
        appear = asset.sound("appear"),
        arrive = asset.sound("log-in"),
        mark = asset.sound("mark"),
        swing = asset.sound("sword-swing"),
        burst = asset.sound("delta-ray-2"),
    },
    -- Effects 0x03, 0x35 and 0x33.
    flash = define.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 },
    mark = define.effect { sprite = asset.sprite("hit-sparks"), anim = 11, palette = 1 },
    slash = define.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
    -- His sword: EXE5's attachment row 3 (0x080C1D62: 0x10103), its animation 1, stepping while dimmed.
    attach = attachment.attach,
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    sword_anim = 1,
    self_type = collision.slash,
    target = collision.hits_navis,
    finish_anim = 0x11,
    burst = function(owner: Object, pos: Vec3, flip: number): Object?
        return follow_effect.spawn(owner, pos, follow_effect.looks.burst, flip)
    end,
    flashes = false,
    candidate_x = common.plus_one,
    tracks = true,
}
