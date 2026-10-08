--!strict
-- EXE4's swords' parts, for the shared slash (@exelib/swords/slash; EXE4's action 0x0A, 0x080EB51E, EXE6's and EXE5's
-- code but for its `SlashGame`): the blade (attachment row 3, 0c-00 in palette 0), held in the related slot as EXE4's
-- own attachment, its animation the navi's number (NaviStats +0x23, MegaMan's 0; ProtoSoul, soul 7, swings blade 13
-- and takes its hit modifiers from 0x080EB7FE: the souls' to wire, docs/design/exe4-map.md §18 item 70); a swing that
-- restarts no form overlay and a recovery that leaves the moving flag. Each chip composes its slash from its variant's
-- rows (`swords.slash { ... }`): the hit (0x080EB7B0's region word: the region, the target and self collision rows),
-- its hit modifier (0x080EB7F1), the slash's effect (0x080EB7E4, a row of the effect table 0x080E1698: the chip's
-- own, `new.effect` of the slash sprites 0c-14 and 0c-15), the swing's
-- sound (0x8E; variants 5 and 11 0xB4), the swings (variant 11's two, 0x080EB5E4), a second hit on the same panel
-- (variant 10's: 0x0705FF01) and a step (the chip's parameter +0x10).

local attachment = require("../objects/attachment")
local slash = require("@exelib/swords/slash")

local swords = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

-- The blade a navi holds (attachment row 3, 0x080CD524: 0c-00 in palette 0).
swords.blade = attachment.look { sprite = asset.sprite("sword"), palette = 0 }

-- The swings' sounds (0x080EB612).
swords.swing = asset.sound("sword-swing")
swords.big_swing = asset.sound("big-sword-swing")

local GAME: SlashGame = {
    -- 0x080EB624: the blade by the navi's number.
    blade_anim = function(_me: Object): number
        return NAVI_NUMBER
    end,
    -- 0x080EB590, 0x080EB676: the afterimages of the navi, of its sprite in its form (0x0800B90A).
    navi_afterimage = function(me: Object): SpriteAsset
        return battle.navi(me.alliance).form.sprite
    end,
    hold = function(me: Object, blade: AttachmentLook, anim: number)
        attachment.attach(me, blade, "related", anim)
    end,
    refreshes_form_overlay = false,
    recovery_clears_moving = false,
}

-- A chip's slash.
function swords.slash(spec: SlashSpec): Action
    return slash.action(spec, attachment.kind, GAME)
end

return swords
