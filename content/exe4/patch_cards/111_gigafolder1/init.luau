--!strict
-- The patch card GigaFolder1 (111, ギガフォルダ1), slot 0A's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: SlottedPatchCard = {
    slot = cards.SLOT_0A,
    effects = {
        cards.giga_folder(1),
    },
}

return { ["111-gigafolder1"] = card }
