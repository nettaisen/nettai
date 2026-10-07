--!strict
-- The patch card GigaFolder1 (111, ギガフォルダ1).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards")

local card: PatchCard = {
    effects = {
        cards.giga_folder(1),
    },
}

return { ["111-gigafolder1"] = card }
