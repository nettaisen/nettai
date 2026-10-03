--!strict
-- BN5's seeds (attack object #0x4A, 0x080CEC04, thrown by action 0x12's subtypes 9, 12, 13 and 14, 0x080CEE02): the
-- shared seed (@common/bombs/seed, BN6's code) with BN5's look: the bomb's sprite in animation 9, BN5's rows 4 and 5,
-- hit modifier 0, no spark, null-element; a splash (effect 1) in a hole, a puff (effect 0x14) when the battle ends.
-- What differs between them is the record's Param1, which indexes its tables (0x080CEDD0, 0x080CEDD4, 0x080CEDC8)
-- and is its palette: IceSeed's ice (0), SeaSeed's sea (1), GrasSeed's grass (2) and LavaSeed's lava (3), with the
-- sprout of their panels (effects 0x29, 0x54, 0x55) and their sounds. The navi holds the bomb in the seed's palette
-- (the attachment table's rows 0x27 to 0x2A), in animation 8.

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@common/bombs/seed")

local seed = {}

local SPRITE = asset.sprite("bn5:bomb")
local SPROUT = asset.sprite("bn5:grab-shot")

seed.kind = common.make_kind("bn5:seed", {
    sprite = SPRITE,
    anim = 9,
    is = collision.attack,
    hits = collision.hits_navis,
    hit_mod = 0,
    splash = define.effect { sprite = asset.sprite("bn5:small-puff"), anim = 0, palette = 0 },
    puff = define.effect { sprite = asset.sprite("bn5:puff"), anim = 2, palette = 0 },
})

-- The seeds by Param1: the panel, the sprout (effect 0x29 twice, 0x54, 0x55) and the sound.
seed.variants = {
    ice = common.variant {
        element = 0,
        palette = 0,
        panel = "ice",
        effect = define.effect { sprite = SPROUT, anim = 1, palette = 0 },
        sound = asset.sound("bn5:freeze"),
    },
    sea = common.variant {
        element = 0,
        palette = 1,
        panel = "sea",
        effect = define.effect { sprite = SPROUT, anim = 1, palette = 0 },
        sound = asset.sound("bn5:sand-worm"),
    },
    grass = common.variant {
        element = 0,
        palette = 2,
        panel = "grass",
        effect = define.effect { sprite = SPROUT, anim = 1, palette = 8 },
        sound = asset.sound("bn5:grass"),
    },
    lava = common.variant {
        element = 0,
        palette = 3,
        panel = "lava",
        effect = define.effect { sprite = SPROUT, anim = 1, palette = 0xC },
        sound = asset.sound("bn5:panel-volcano"),
    },
}

-- What the navi holds throwing a seed of `variant`: the bomb in its palette.
function seed.held(variant: SeedVariant): AttachmentLook
    return attachment.look { sprite = SPRITE, palette = variant.palette }
end

-- A thrower of seeds of `variant`.
function seed.thrower(variant: SeedVariant): Thrower
    return common.thrower(seed.kind, variant)
end

return seed
