--!strict
-- BN5's buster arm (0x080EBAAE and 0x080EBABE, BN6's `sub_80EB562` and `sub_80EB572`: the same shape, BN5's own
-- choices): the arm a navi raises for a shot, held in its overlay slot (an attachment). By its holder:
--
-- - a virus: the buster arm (the attachment table's row 0x23);
-- - an enemy navi: its own arm (row 0x2B) in its AI index's animation, palette 13; AI index 0x16 the buster arm in
--   palette 14;
-- - a player of another AI index: the navi arm, its animation the AI index less 1, palette 25 less that when the
--   charge is disabled; AI index 0x17 the buster arm in palette 14;
-- - MegaMan (a player of AI index 0): in a soul, the buster arm in the soul's animation (row 0x33 while the chaos
--   charge is armed, AIData+0x12, palette 10 with the charge disabled); in none, by NaviStats +0x4C: 0, the buster
--   arm (palette 14 with the charge disabled, else 0x1B plus his element); 1, the arm in animation 13; more, row
--   0x33 in animation 13 (palette 1 with the charge disabled).
--
-- (NaviStats +0x4C is Hub Style, the navi's `hub_style`: Team ProtoMan's patch card 111 sets 1, Team Colonel's 2.)

local attachment = require("../objects/attachment")

local arm = {}

local ARM = asset.sprite("buster-arm")
-- The attachment table's rows 0x23 (the buster arm), 0x2B (an enemy navi's arm) and 0x33 (the buster arm in the
-- chaos palette).
arm.looks = {
    buster = attachment.look { sprite = ARM, palette = 0 },
    navi = attachment.look { sprite = asset.sprite("navi-arm"), palette = 0 },
    chaos = attachment.look { sprite = ARM, palette = 14 },
}

-- NaviStats +0x4C (Hub Style: 0, 1 or 2), and AIData+0x12, the Chaos Unison charge armed.
local function dark_arm(navi: Object): number
    return battle.navi(navi.alliance).hub_style
end
local function chaos_charge(navi: Object): number
    return if navi.chaos_armed then 1 else 0
end

-- 0x080EBABE: the arm `navi` calls for, held in `which` (its r9, Param3, by 16: the arm animates while dimmed) of
-- `holder` (its r8, a slot's address: `navi`'s own unless said).
function arm.raise_for(navi: Object, which: Slot, while_dimmed: boolean, holder: Object?)
    local look, anim, palette = arm.looks.buster, 0, 0
    local kind = navi.actor_type
    if kind == "navi" then
        if navi.ai_index == 0x16 then
            palette = 14
        else
            look, anim, palette = arm.looks.navi, navi.ai_index - 1, 13
        end
    elseif kind == "player" then
        local ai = navi.ai_index
        if ai ~= 0 then
            if ai == 0x17 then
                palette = 14
            else
                look, anim = arm.looks.navi, ai - 1
                if navi:navi_state("no_charge") then
                    palette = 25 - anim
                end
            end
        else
            local stats = battle.navi(navi.alliance)
            local soul = stats.form.soul
            if soul then
                anim = soul.number
                if chaos_charge(navi) ~= 0 then
                    look = arm.looks.chaos
                    if navi:navi_state("no_charge") then
                        palette = 10
                    end
                end
            else
                local dark = dark_arm(navi)
                if dark == 0 then
                    if navi:navi_state("no_charge") then
                        palette = 14
                    elseif stats.element ~= 0 then
                        palette = 0x1B + stats.element
                    end
                elseif navi:navi_state("no_charge") then
                    anim, palette = 13, 1
                else
                    anim = 13
                    if dark ~= 1 then
                        look = arm.looks.chaos
                    end
                end
            end
        end
    end
    -- (The spawn word: the row, the animation by 8, Param3 by 16 and the palette by 24: a negative animation
    -- borrows from the bytes above it.)
    local word = int.u32(int.shl(anim, 8) + int.shl(if while_dimmed then 1 else 0, 16) + int.shl(palette, 24))
    attachment.attach(holder or navi, look, which, {
        anim = bit32.band(bit32.rshift(word, 8), 0xFF),
        while_dimmed = bit32.band(bit32.rshift(word, 16), 0xFF) ~= 0,
        palette_add = bit32.rshift(word, 24),
    })
end

-- 0x080EBAAE: raise the arm, in the overlay slot.
function arm.raise(navi: Object)
    arm.raise_for(navi, "overlay", false)
end

return arm
