--!strict
-- EXE4's swords' parts, for the shared slash (@exelib/swords/slash; EXE4's action 0x0A, 0x080EB51E, EXE6's and EXE5's
-- code but for its `SlashGame`): the blade (attachment row 3, 0c-00 in palette 0), held in the related slot as EXE4's
-- own attachment, its animation the navi's number (NaviStats +0x23, MegaMan's 0; ProtoSoul, soul 7, swings blade 13
-- and takes its hit modifiers from 0x080EB7FE: the souls' to wire, docs/design/exe4-map.md §18 item 70); a swing that
-- restarts no form overlay and a recovery that leaves the moving flag. Each chip composes its slash from its variant's
-- rows (`swords.slash { ... }`): the hit (0x080EB7B0's region word: the region, the target and self collision rows),
-- its hit modifier (0x080EB7F1), the slash's effect (0x080EB7E4, a row of the effect table 0x080E1698), the swing's
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

local SLASH, BIG_SLASH = asset.sprite("sword-slash"), asset.sprite("big-slash")

-- The slashes' effects by the variants' rows of the effect table (0x080EB7E4: 0x080E1698's rows).
swords.effects = {
    -- 0x17 to 0x19: over a column, two panels, one panel.
    wide = new.effect { sprite = SLASH, anim = 0, palette = 0 },
    long = new.effect { sprite = SLASH, anim = 1, palette = 0 },
    slash = new.effect { sprite = SLASH, anim = 2, palette = 0 },
    -- 0x1A and 0x1B: the wide and long ones in palette 5.
    wide_blade = new.effect { sprite = SLASH, anim = 0, palette = 5 },
    long_blade = new.effect { sprite = SLASH, anim = 1, palette = 5 },
    -- 0x1D: a big slash over two columns; 0x1E: the dark one.
    big = new.effect { sprite = BIG_SLASH, anim = 0, palette = 0 },
    dark = new.effect { sprite = BIG_SLASH, anim = 0, palette = 2 },
}

local GAME: SlashGame = {
    -- 0x080EB624: the blade by the navi's number.
    blade_anim = function(_me: Object): number
        return NAVI_NUMBER
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
