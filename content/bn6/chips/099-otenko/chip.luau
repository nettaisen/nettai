--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): Otenko's record, whose
-- extra flags the chip definitions can't express yet, runs the chip's `dimming` hook
-- (chips/otenko) as dimming chip subtype 18.

local otenko = require("../otenko/chip")

return { dimming_chip = otenko.dimming }
