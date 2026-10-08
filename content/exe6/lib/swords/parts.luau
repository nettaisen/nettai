--!strict
-- What EXE6's swords share (docs/design/content-model-v2.md §5.2): the
-- plain blade, their swings' sounds, and the blade the user holds while it
-- swings (the shared parts, @exelib/swords/parts, with EXE6's attachment).
-- Each sword states the slash drawn over the panels it hits.

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/swords/parts")
local kind = require("../../rules/forms/kind")

local parts = {}

-- What a sword's hit is and hits: a slash that reaches navis.
parts.slashes_navis = { is = collision.slash, hits = collision.hits_navis }

-- The plain blade the navi holds (the original's attachment row 3), which most swords hold. (The elemental swords'
-- and BambSwrd's are their own: rows 0x19 to 0x1C.)
parts.blade = attachment.look { sprite = asset.sprite("sword"), palette = 0 }

-- The swings' sounds: most swords', and the big swords' (LifeSrd, DrkSword, DblDream). (The slashes over the panels
-- each sword hits are its own.)
parts.swing = asset.sound("sword-swing")
parts.big_swing = asset.sound("big-sword-swing")

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
