--!strict
-- The patch card All Guard 1/2 (76, オールガード1/2).
-- Written by tools/exe4/gen_patch_cards.py (bn6battle-verify) from the EXE4 ROMs.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    effects = {
        cards.pair(3, "077-all-guard-2-2", { cards.all_guard(), cards.bug("hp", 3) }),
    },
}

return { ["076-all-guard-1-2"] = card }
