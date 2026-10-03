--!strict
-- BN5's ProtoMan, the navi of its ProtoMan chips: BN6's navi (@bn6/chips/protoman/navi; BN5's the same code but for
-- his sprite, 08-01, BN6's 08-0b) with BN5's look: his sword BN5's attachment row 3 (BN6's 0x11, the same look) in
-- its animation 1 (BN6 0xB, 0x080C199C's rows), his slash BN5's effect 0x33 (BN6's 0x27, the same row), his palette by
-- the chip's Param1 (0x080C1714: 0 for ProtoMan's and ProtoMnSP's 0 and 3, 3 for ProtoMnDS's 4) and his spawner
-- BN5's 0x080C1A00.

local attachment = require("@bn6/objects/attachment/attachment")
local navi = require("@bn6/chips/protoman/navi")

local protoman = {}

local SWORD = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 }
local SLASH = define.effect { sprite = asset.sprite("bn5:sword-slash"), anim = 0, palette = 0 }

local function look(palette: number): ProtoManLook
    return navi.look {
        sprite = asset.sprite("bn5:protoman"),
        palette = palette,
        sword = SWORD,
        sword_anim = 1,
        slash = SLASH,
        spawner_address = 0x080C1A01,
    }
end

-- ProtoMan's and ProtoMnSP's ProtoMan.
protoman.summon = navi.summon_with(look(0))
-- ProtoMnDS's: his dark palette.
protoman.summon_dark = navi.summon_with(look(3))

return protoman
