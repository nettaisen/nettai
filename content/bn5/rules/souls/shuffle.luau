--!strict
-- SearchSoul's Shuffle on BN5's custom screen (docs/design/bn5-map.md §15.8): a button two cells wide on slots 8
-- and 9 (0x08023CF8: the slots' types 4 and 5, its right neighbor the special slot, its left the eighth chip's),
-- three times a screen (3 less the screen's shuffles so far, +0x16, which each screen's open zeroes). A on it
-- (0x080249B0, with no sound of its own) deals the chips not picked again: the screen's state 0x28 (0x08023488),
-- the framework's re-deal, which BN6's ChpShufl kept. BN5's keeps some of the hand in the hand (0x080253E8, the table
-- 0x080254C8 by how many of the hand it deals again: rules/custom-screen's `redeal_kept`): those are shuffled among
-- themselves, and all but the first kept with the rest of the folder.
--
-- The slots are set by the soul the emotion routine leaves in r1 (0x08023C3C: NaviStats +0x2C), which in battle
-- mode 1 is the stats' address instead (0x080127C0): no button there.

local shuffle = {}

-- The soul's number (NaviStats +0x2C).
local SEARCHSOUL = 3

shuffle.button = {
    slot = 8,
    cells = 2,
    uses = 3,
    right = 11,
    left = 7,
    shown = function(side: number): boolean
        local soul = battle.navi(side).form.soul
        return soul ~= nil and soul.number == SEARCHSOUL and battle.mode() ~= 1
    end,
    pressed = function(side: number)
        if custom.cursor_state(side) == "selectable" then
            custom.redeal(side)
        else
            custom.refuse(side)
        end
    end,
} :: ButtonSpec

return shuffle
