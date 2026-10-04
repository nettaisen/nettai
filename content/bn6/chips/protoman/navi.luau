--!strict
-- ProtoMan, the navi of the ProtoMan chips (the original's actor object
-- #0x3C, `sub_80C26F0`): the shared ProtoMan (@exelib/protoman/navi), BN6's
-- kind and look. BN6's look is `protoman.bn6`, which `protoman.summon`
-- brings.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/protoman/navi")
local parts = require("../../lib/swords/parts")

local protoman = {}

-- A game's ProtoMan's look.
protoman.look = common.look

-- BN6's: his sprite; his palette (`dword_80C276C`: 0 for each of the
-- chips' levels); his sword (`byte_80C29F0`, the same row for each of the
-- chips' levels: the original's attachment 0x11, the swords' blade), its
-- animation 0xB, animating while dimmed; his slash's effect (0x27: the
-- swords' slash over a column); and his spawner's address, which the
-- controller calls through r3 and his position takes (his init sets it
-- all).
protoman.bn6 = protoman.look {
    sprite = asset.sprite("bn6:protoman"),
    palette = 0,
    sword = parts.blades.sword,
    sword_anim = 0xB,
    slash = parts.effects.wide,
    spawner_address = 0x080C2A4D,
}

local made = common.make {
    id = "bn6:protoman/navi",
    appear_sound = asset.sound("bn6:appear"),
    swing_sound = parts.sounds.swing,
    attachment = attachment.kind,
    target = collision.hits_navis,
    self_type = collision.slash,
}
protoman.kind = made.kind

-- `sub_80C2A4C`: ProtoMan of `look` (the ProtoMan chips' navi hook).
protoman.summon_with = made.summon_with

-- BN6's ProtoMan.
protoman.summon = protoman.summon_with(protoman.bn6)

return protoman
