--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): Meteors' record, which
-- SunMoon's recipe names, runs the chip's `dimming` hook (chips/meteors) as dimming chip
-- subtype 16. It goes when the recipes name chips by definition.

local meteors = require("../meteors/chip")

return { dimming_chip = meteors.dimming }
