--!strict
-- Geddon's quake (the original's effect object #0x1E, `sub_80E25D0`;
-- docs/engine/dimming-chip-effects.md §12): it lists the field's solid
-- panels, shuffles them, and changes one every 8 ticks, the last listed
-- first (Geddon breaks them, the capsules crack them, a third change no
-- chip uses poisons them), with a puff over each that changed; then it
-- lets its holder go on. Geddon's and the capsules' controller
-- (./controller) spawns it. The behavior is BN5's too
-- (@exelib/geddon/quake), made here with BN6's look.

local common = require("@exelib/geddon/quake")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")

return common.make {
    id = "bn6:geddon/quake",
    puff = define.effect { sprite = asset.sprite("bn6:small-puff"), anim = 1, palette = 0 },
    bubble = rising_bubble.kind,
    sounds = { puff = asset.sound("bn6:panel-crack"), bubble = asset.sound("bn6:panel-poison") },
}
