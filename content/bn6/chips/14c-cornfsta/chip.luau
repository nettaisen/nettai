--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- record of CornFsta (dimming chip subtype 29; a Program Advance) runs its
-- chip's `dimming` hook (chips/cornfsta). It goes when chip definitions
-- take Program Advance recipes.

local cornfsta = require("../cornfsta/chip")

return { dimming_chip = cornfsta.dimming }
