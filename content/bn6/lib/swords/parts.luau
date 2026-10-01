--!strict
-- What the swords share (docs/design/content-model-v2.md §5.2): their
-- blades, the slashes drawn over the panels they hit, their swings'
-- sounds, and the blade the user holds while it swings (its animation
-- follows the user's form).

local attachment = require("../../objects/attachment/attachment")
local collision = require("../../rules/collision")

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
    -- swords' (in their colours).
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

-- `sub_80EBAE8`: the blade's animation, which matches the user's arm: a
-- navi AI's (under 0xD), a form's (AI index 0x13 has animation 0xE), or
-- for MegaMan's base AI, 0 or a Beast form's (0xC for Gregar's, 0xD for
-- Falzar's).
function parts.blade_anim(me: Object): number
    local kind, ai = me.actor_type, me.ai_index
    if kind == "navi" then
        return if ai < 0x0D then ai else 0
    elseif kind == "player" then
        if ai ~= 0 then
            return if ai == 0x13 then 0x0E else ai
        end
        return parts.base_blade_anim(me.alliance)
    end
    return 0
end

-- The same for MegaMan's base AI, by side `side`'s form: 0, or a Beast
-- form's (0xC for Gregar's, 0xD for Falzar's). (CrosOver's MegaMan, whose
-- user may be anyone, picks his blade as MegaMan's base AI.)
function parts.base_blade_anim(side: number): number
    local stats = battle.navi(side)
    if not stats.beast then
        return 0
    end
    return if stats.form.game == "gregar" then 0x0C else 0x0D
end

-- `sub_80EBB78`: added to the blade's palette: 0xC for navi AI 0x12's
-- version 4, else 0.
function parts.blade_palette(me: Object): number
    if me.actor_type == "navi" and me.ai_index == 0x12 then
        -- The game compares AIData+0x16, a navi AI's version, which this
        -- engine's actors (players) don't have.
        error("sub_80EBB78: navi AI 0x12's version (AIData+0x16) is not modelled")
    end
    return 0
end

-- `sub_80B8E30` with a blade: hold it (in the first related slot), its
-- palette plus `palette_add`.
function parts.hold(me: Object, blade: AttachmentLook, palette_add: number): Object?
    return attachment.attach(me, blade, "related", { anim = parts.blade_anim(me), palette_add = palette_add })
end

-- Let go of the blade (and anything in the overlay slot): what is attached
-- there ends itself.
function parts.drop(me: Object)
    me.related1 = nil
    me.overlay = nil
end

return parts
