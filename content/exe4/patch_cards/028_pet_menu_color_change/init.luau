--!strict
-- The patch card PET Menu Color Change (28, PET画面カラー変更).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

-- (No effect in battle: a card of the PET's menu.)
local card: PatchCard = { effects = {} }

return { ["028-pet-menu-color-change"] = card }
