--!strict
-- LifeSrd: a big slash on the two columns ahead; the Program Advance of Sword, WideSwrd, LongSwrd (or
-- WideBlde, LongBlde).
--
-- Its record stays the pack's for now (its Program Advance recipes come to definitions with the
-- chips they name): this is its action, which registration by number runs (docs/design/content-
-- model-v2.md §5.1, "As built").

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")

return {
    action = slash.action {
        id = "lifesrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.big,
        sound = parts.sounds.big_swing,
    },
}
