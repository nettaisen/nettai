--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x28 runs
-- this module, which runs the chips' own draw (chips/trnarrw). TrnArrw1 to
-- 3 are definitions; the records that still reach it are TrnArrw2's and
-- TrnArrw3's, which the Cross special picks by number (berserk.rs
-- `CROSS_SPECIAL_CHIPS`). The three draw alike (their records differ only
-- in damage, which the attack holds), so the first one's runs for each. It
-- goes when that list names chips (phase C, step 10).

local trnarrw = require("../trnarrw/chips")

return {
    state = trnarrw.STATE,
    update = trnarrw[1].action.update,
}
