--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- record of DarkInvs (dimming chip subtype 38; a dark chip, with a
-- substitute) runs its chip's `dimming` hook (chips/darkinvs). The
-- subtype's other chips (HubBatc, BugRSwrd, BgDthThd and the arms) are
-- definitions and don't come here: their rows (`off_80E95C8` by the first
-- parameter byte) name a weapon by its routine's number, which only a
-- definition can name. It goes when definitions take dark chips'
-- substitutes.

local darkinvs = require("../darkinvs/chip")
local navi_boost = require("../../lib/navi-boost/controller")

local function dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local p = spec.params
    local row = p[1]
    if row == 0 then
        return navi_boost.hub(user, spec)
    elseif row == 1 or row == 2 then
        error(string.format("a record of navi-changing row %d names its weapon by number: the chip's definition names it", row))
    elseif row == 3 then
        return darkinvs.dimming(user, spec)
    end
    error(string.format("navi-changing chip Param1 %d reads past off_80E95C8", row))
end

return { dimming_chip = dimming_chip }
