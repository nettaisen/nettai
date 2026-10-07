--!strict
-- EXE5's seeds (attack object #0x4A, 0x080CEC04, thrown by action 0x12's subtypes 9, 12, 13 and 14, 0x080CEE02): the
-- shared seed (@exelib/bombs/seed, EXE6's code) with EXE5's look: the bomb's sprite in animation 9, EXE5's rows 4 and 5,
-- hit modifier 0, no spark, null-element; a splash (effect 1) in a hole, a puff (effect 0x14) when the battle ends.
-- Each chip states its own seed (`seed.variant`): what the original reads from its tables by the record's Param1
-- (0x080CEDD0, 0x080CEDD4, 0x080CEDC8), the panel, the sprout (effects 0x29, 0x54, 0x55) and the sound, and the
-- palette, which is that Param1 (IceSeed's 0, SeaSeed's 1, GrasSeed's 2, LavaSeed's 3). The navi holds the bomb in
-- the seed's palette (the attachment table's rows 0x27 to 0x2A), in animation 8.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/bombs/seed")

local seed = {}

local SPRITE = asset.sprite("bomb")

seed.kind = common.make_kind("seed", {
    sprite = SPRITE,
    anim = 9,
    is = collision.attack,
    hits = collision.hits_navis,
    hit_mod = 0,
    splash = new.effect { sprite = asset.sprite("small-puff"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
})

-- A seed variant: its element, palette, panel, sprout and sound.
seed.variant = common.variant

-- What the navi holds throwing a seed of `variant`: the bomb in its palette.
function seed.held(variant: SeedVariant): AttachmentLook
    return attachment.look { sprite = SPRITE, palette = variant.palette }
end

-- A thrower of seeds of `variant`.
function seed.thrower(variant: SeedVariant): Thrower
    return common.thrower(seed.kind, variant)
end

return seed
