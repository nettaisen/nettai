--!strict
-- What the swords share, as BN5 and BN6 share it
-- (docs/design/content-model-v2.md §5.2): the blade the user holds while it
-- swings (its animation follows the user's form). The blades, slashes and
-- sounds themselves are each game's (BN6's lib/swords/parts, BN5's
-- lib/swords).

local attachment = require("../attachment")

local parts = {}

-- `sub_80EBAE8`: the blade's animation, which matches the user's arm: a
-- navi AI's (under 0xD), a form's (AI index 0x13 has animation 0xE), or
-- for MegaMan's base AI, `base`'s by the user's side (none: 0; BN6's gives
-- its Beast forms' arms, its lib/swords/parts).
function parts.arm_anim(me: Object, base: ((side: number) -> number)?): number
    local kind, ai = me.actor_type, me.ai_index
    if kind == "navi" then
        return if ai < 0x0D then ai else 0
    elseif kind == "player" then
        if ai ~= 0 then
            return if ai == 0x13 then 0x0E else ai
        end
        return if base ~= nil then base(me.alliance) else 0
    end
    return 0
end

-- The same, MegaMan's base AI's being 0.
function parts.blade_anim(me: Object): number
    return parts.arm_anim(me, nil)
end

-- `sub_80EBB78`: added to the blade's palette: 0xC for navi AI 0x12's
-- version 4, else 0.
function parts.blade_palette(me: Object): number
    if me.actor_type == "navi" and me.ai_index == 0x12 then
        -- The game compares AIData+0x16, a navi AI's version, which this
        -- engine's actors (players) don't have.
        error("sub_80EBB78: navi AI 0x12's version (AIData+0x16) is not modeled")
    end
    return 0
end

-- `sub_80B8E30` with a blade: hold it (an attachment of `kind`, a game's,
-- in the first related slot), its palette plus `palette_add`, in animation
-- `anim` (none: `blade_anim`'s).
function parts.hold(kind: Kind, me: Object, blade: AttachmentLook, palette_add: number, anim: number?): Object?
    return attachment.attach(kind, me, blade, "related", { anim = anim or parts.blade_anim(me), palette_add = palette_add })
end

-- Let go of the blade (and anything in the overlay slot): what is attached
-- there ends itself.
function parts.drop(me: Object)
    me.related1 = nil
    me.overlay = nil
end

return parts
