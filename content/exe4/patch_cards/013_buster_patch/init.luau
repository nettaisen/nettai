--!strict
-- The patch card Buster Patch (13, バスターを改造), slot 0A's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local buster_patches = require("../navis/megaman/weapons/buster_patches")

local card: SlottedPatchCard = {
    slot = cards.SLOT_0A,
    effects = {
        cards.b_button(buster_patches.grass),
    },
}

return { ["013-buster-patch"] = card }
