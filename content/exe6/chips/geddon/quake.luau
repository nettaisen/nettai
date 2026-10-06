--!strict
-- Geddon's quake (the original's effect object #0x1E, `sub_80E25D0`;
-- docs/engine/dimming-chip-effects.md §12): it lists the field's solid
-- panels, shuffles them, and changes one every 8 ticks, the last listed
-- first (Geddon breaks them, the capsules crack them, a third change no
-- chip uses poisons them), with a puff over each that changed; then it
-- lets its holder go on. Geddon's and the capsules' controller
-- (./controller) spawns it. The behavior is EXE5's too
-- (@exelib/geddon/quake), made here with EXE6's look.

local common = require("@exelib/geddon/quake")
local rising_bubble = require("../../objects/rising_bubble")

return common.make {
    id = "geddon/quake",
    puff = new.effect { sprite = asset.sprite("small-puff"), anim = 1, palette = 0 },
    bubble = rising_bubble.kind,
    sounds = { puff = asset.sound("panel-crack"), bubble = asset.sound("panel-poison") },
}
