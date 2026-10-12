--!strict
-- The patch card Search Soul (124, サーチソウル), slot 0E's.
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")
local searchsoul = require("../navis/megaman/forms/searchsoul").searchsoul

local card: SlottedPatchCard = {
    slot = cards.SLOT_0E,
    effects = {
        cards.soul(searchsoul),
        cards.bug("hp", 2),
    },
}

return { ["124-search-soul"] = card }
