--!strict
-- The patch card Fire Soul (125, ファイアソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local firesoul = require("../navis/megaman/forms/firesoul").firesoul

local card: SlottedPatchCard = {
    slot = cards.SLOT_0E,
    effects = {
        cards.soul(firesoul),
        cards.bug("hp", 2),
    },
}

return { ["125-fire-soul"] = card }
