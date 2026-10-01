--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- record of NumbrBl (dimming chip subtype 22; its damage is a formula)
-- runs its chip's `dimming` hook (chips/numbrbl). It goes when chip
-- definitions take damage formulas.

local numbrbl = require("../numbrbl/chip")

return { dimming_chip = numbrbl.dimming }
