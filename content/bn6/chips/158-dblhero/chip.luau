--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- record of DblHero (dimming chip subtype 30; a Program Advance) runs its
-- chip's `dimming` hook (chips/dblhero). It goes when chip definitions
-- take Program Advance recipes.

local dblhero = require("../dblhero/chip")

return { dimming_chip = dblhero.dimming }
