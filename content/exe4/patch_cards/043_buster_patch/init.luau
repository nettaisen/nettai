--!strict
-- The patch card Buster Patch (43, バスターを改造).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local buster_patches = require("../navis/megaman/weapons/buster_patches")

local card: PatchCard = {
    effects = {
        cards.b_button(buster_patches.crack),
    },
}

return { ["043-buster-patch"] = card }
