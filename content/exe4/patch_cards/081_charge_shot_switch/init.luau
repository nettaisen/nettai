--!strict
-- The patch card Charge Shot Switch (81, チャージ武器を変更).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local zap_ring = require("../navis/megaman/weapons/zap_ring")

local card: PatchCard = {
    effects = {
        cards.charged_shot(zap_ring.weapon),
    },
}

return { ["081-charge-shot-switch"] = card }
