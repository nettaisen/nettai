--!strict
-- What EXE6's swords share (docs/design/content-model-v2.md §5.2): their
-- blades, the slashes drawn over the panels they hit, their swings'
-- sounds, and the blade the user holds while it swings (the shared parts,
-- @exelib/swords/parts, with EXE6's attachment).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/swords/parts")
local kind = require("../../rules/forms/kind")

local parts = {}

-- What a sword's hit is and hits: a slash that reaches navis.
parts.slashes_navis = { is = collision.slash, hits = collision.hits_navis }

-- The blades (the original's attachment rows 3, 0x19 to 0x1C).
parts.blades = {
    sword = attachment.look { sprite = asset.sprite("sword"), palette = 0 },
    fire = attachment.look { sprite = asset.sprite("fire-sword"), palette = 0 },
    aqua = attachment.look { sprite = asset.sprite("aqua-sword"), palette = 0 },
    elec = attachment.look { sprite = asset.sprite("elec-sword"), palette = 0 },
    bamboo = attachment.look { sprite = asset.sprite("sword"), palette = 2 },
}

-- The slashes (effects 0x16 to 0x1C, 0x25, 0x2D, 0x5F).
local SLASH, BIG = asset.sprite("sword-slash"), asset.sprite("big-slash")
parts.effects = {
    -- Sword's, over the panel ahead.
    slash = define.effect { sprite = SLASH, anim = 2, palette = 0 },
    -- Over a column: WideSwrd's, StepSwrd's, the elemental and machine
    -- swords' (in their colors).
    wide = define.effect { sprite = SLASH, anim = 0, palette = 0 },
    -- Over two panels: LongSwrd's.
    long = define.effect { sprite = SLASH, anim = 1, palette = 0 },
    wide_blade = define.effect { sprite = SLASH, anim = 0, palette = 5 },
    long_blade = define.effect { sprite = SLASH, anim = 1, palette = 5 },
    -- Over three panels: FtrSword's.
    line = define.effect { sprite = SLASH, anim = 3, palette = 0 },
    muramasa = define.effect { sprite = SLASH, anim = 1, palette = 6 },
    -- LifeSrd's and DblDream's; DrkSword's, dark.
    big = define.effect { sprite = BIG, anim = 0, palette = 0 },
    dark = define.effect { sprite = BIG, anim = 0, palette = 2 },
    -- CrosSwrd's X.
    cross = define.effect { sprite = asset.sprite("cross-slash"), anim = 0, palette = 0 },
}

-- The swings: most swords', and the big swords' (LifeSrd, DrkSword,
-- DblDream).
parts.sounds = {
    swing = asset.sound("sword-swing"),
    big_swing = asset.sound("big-sword-swing"),
}

-- MegaMan's base AI's blade, by side `side`'s form: 0, or a Beast form's
-- (0xC for Gregar's, 0xD for Falzar's). (CrosOver's MegaMan, whose user may
-- be anyone, picks his blade as MegaMan's base AI.)
function parts.base_blade_anim(side: number): number
    if not kind.beast(side) then
        return 0
    end
    return if battle.navi(side).form.version == "gregar" then 0x0C else 0x0D
end

-- `sub_80EBAE8`: the blade's animation, which matches the user's arm.
function parts.blade_anim(me: Object): number
    return common.arm_anim(me, parts.base_blade_anim)
end
-- `sub_80EBB78`: added to the blade's palette.
parts.blade_palette = common.blade_palette

-- `sub_80B8E30` with a blade: hold it (in the first related slot), its
-- palette plus `palette_add`, in animation `anim` (none: `blade_anim`'s).
function parts.hold(me: Object, blade: AttachmentLook, palette_add: number, anim: number?): Object?
    return common.hold(attachment.kind, me, blade, palette_add, anim or parts.blade_anim(me))
end

-- Let go of the blade (and anything in the overlay slot).
parts.drop = common.drop

return parts
