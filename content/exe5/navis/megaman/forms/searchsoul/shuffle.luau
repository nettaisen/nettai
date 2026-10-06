--!strict
-- SearchSoul's Shuffle on EXE5's custom screen (docs/design/exe5-map.md §15.8): what the soul adds to the screen (its
-- form's `custom`, which the souls system gathers: rules/souls/custom). A button two cells wide on slots 8 and 9
-- (0x08023CF8: the slots' types 4 and 5, its right neighbor the special slot, its left the eighth chip's), three
-- times a screen (3 less the screen's shuffles so far, +0x16, which each screen's open zeroes). A on it (0x080249B0,
-- with no sound of its own) deals the chips not picked again: the screen's state 0x28 (0x08023488), the framework's
-- re-deal, which EXE6's ChpShufl kept. EXE5's keeps some of the hand in the hand (0x080253E8, the table 0x080254C8 by
-- how many of the hand it deals again: rules/custom_screen's `redeal_kept`): those are shuffled among themselves,
-- and all but the first kept with the rest of the folder.

local shuffle = {}

local custom_screen: SoulCustom = {
    buttons = {
        -- (By the name the frontend draws the pack's re-deal button for.)
        redeal = {
            slot = 8,
            cells = 2,
            uses = 3,
            right = 11,
            left = 7,
            pressed = function(side: number, _s: any)
                if custom.cursor_state(side) == "selectable" then
                    custom.redeal(side)
                else
                    custom.refuse(side)
                end
            end,
        },
    },
}

shuffle.custom = define.record("soul-custom", custom_screen)

return shuffle
