--!strict
-- The patch card Charge Shot Switch (20, チャージ武器を変更).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local machine_gun = require("../navis/megaman/weapons/machine_gun")

local card: PatchCard = {
    effects = {
        cards.charged_shot(machine_gun.weapon),
        cards.clear_bug("weapon"),
    },
}

return { ["020-charge-shot-switch"] = card }
