--!strict
-- A thrown seed (the original's attack object #0x4F, `sub_80CE24C`):
-- PoisSeed, IceSeed and GrasSeed: the shared seed (@exelib/bombs/seed)
-- with BN6's look. It flies three panels ahead in the bomb's arc and
-- lands: on a solid panel it hits it for a tick, then turns the panels
-- around it into its variant's kind of panel, only its own if the hit
-- landed on something; off the field or on a hole it just ends (a splash
-- in the hole). What differs between the seeds is its variant
-- (`dword_80CE2B0`, `byte_80CE41E`, `byte_80CE421`, `byte_80CE418` and the
-- chip's palette).

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/bombs/seed")
local effects = require("../effects")
local sparks = require("../sparks")

local seed = {}

-- `sub_8011504`: sprite `bomb`, animation 9, with a ground shadow.
local SPRITE = asset.sprite("bn6:bomb")

-- What grows on each panel: a sprout (poison, ice) or a green one (grass)
-- (effects 0x3B, 0x3C).
seed.sprout = define.effect { sprite = asset.sprite("bn6:grab-shot"), anim = 1, palette = 0 }
seed.green_sprout = define.effect { sprite = asset.sprite("bn6:grab-shot"), anim = 1, palette = 8 }

-- What the navi holds while throwing a seed: the bomb's sprite in the
-- seed's colors (the original's attachment rows 0x24, 0x25, 0x26).
function seed.held(palette: number): AttachmentLook
    return attachment.look { sprite = SPRITE, palette = palette }
end

-- A seed variant.
seed.variant = common.variant

seed.kind = common.make_kind("bn6:seed", {
    sprite = SPRITE,
    anim = 9,
    is = collision.attack,
    hits = collision.hits_navis,
    hit_mod = 1,
    spark = sparks.of_element,
    splash = effects.splash,
    puff = effects.puff,
})

-- `sub_80CE44E`: a thrower for `throw.action` that throws a seed of
-- `variant`.
function seed.thrower(variant: SeedVariant): Thrower
    return common.thrower(seed.kind, variant)
end

return seed
