--!strict
-- DiveMan, DiveMan[EX] and DiveMan[SP]: DiveMan comes (./navi).
--
-- Their records stay the pack's for now (the SP chip's damage is a
-- formula, which definitions take in step 10): this is their navi hook,
-- which registration by number runs for all of them
-- (docs/design/content-model-v2.md §5.5).

local navi = require("./navi")

return {
    id = "diveman",
    navi_chip = navi.summon,
}
