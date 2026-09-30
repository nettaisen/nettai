--!strict
-- What the sword actions share (0x13, `sub_80EB776`, and 0x49,
-- `sub_80EEB4C`): the blade their user holds while it swings. Which blade
-- is the chip's data (`data.chips[id].sword`); its animation follows the
-- user's form.

local attachment = require("../objects/attachment/attachment")

local sword = {}

-- The Beast forms (and Beast Over): forms 0xB to 0x18.
function sword.beast(form: number): boolean
    return form >= 0x0B and form <= 0x18
end

-- The chip's sword data.
function sword.data(me: Object): Sword
    local chip = data.chips[me.chip]
    local d = chip and chip.sword
    if not d then
        error(string.format("chip %#x swings a sword without its sword data", me.chip))
    end
    return d
end

-- `sub_80EBAE8`: the blade's animation, which matches the user's arm: a
-- navi AI's (under 0xD), a form's (AI index 0x13 has animation 0xE), or
-- for MegaMan's base AI, 0 or a Beast form's (0xC for Gregar's, 0xD for
-- Falzar's).
function sword.blade_anim(me: Object): number
    local kind, ai = me.actor_type, me.ai_index
    if kind == "navi" then
        return if ai < 0x0D then ai else 0
    elseif kind == "player" then
        if ai ~= 0 then
            return if ai == 0x13 then 0x0E else ai
        end
        local form = battle.navi(me.alliance).form
        if form < 0x0B then
            return 0
        elseif form == 0x0C then
            return 0x0D
        elseif form == 0x17 or form <= 0x11 then
            return 0x0C
        end
        return 0x0D
    end
    return 0
end

-- `sub_80EBB78`: added to the blade's palette: 0xC for navi AI 0x12's
-- version 4, else 0.
function sword.blade_palette(me: Object): number
    if me.actor_type == "navi" and me.ai_index == 0x12 then
        -- The game compares AIData+0x16, a navi AI's version, which this
        -- engine's actors (players) don't have.
        error("sub_80EBB78: navi AI 0x12's version (AIData+0x16) is not modelled")
    end
    return 0
end

-- `sub_80B8E30` with the blade: hold it (in the first related slot) with
-- `palette_add`.
function sword.hold_blade(me: Object, palette_add: number): Object?
    return attachment.spawn(me, {
        kind = sword.data(me).blade,
        anim = sword.blade_anim(me),
        palette_add = palette_add,
    }, "related")
end

-- Let go of the blade (and anything in the overlay slot): what is attached
-- there ends itself.
function sword.drop_blade(me: Object)
    me.related1 = nil
    me.overlay = nil
end

return sword
