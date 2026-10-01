--!strict
-- Registration by number (docs/design/content-model-v2.md §5.4): the
-- records of Invisibl and the second WhiCapsl (dimming chip subtype 1;
-- the ruleset names Invisibl as a dark chip's substitute and finds the
-- chips past 0x170 by number) run their chips' `dimming` hooks
-- (chips/invisibl, chips/whicapsl-invisible), picked by the time their
-- first two parameter bytes give, as the original's controller reads it
-- (`sub_8010474`). It goes when the ruleset names those chips by trait
-- (phase C, step 10).

local invisibl = require("../invisibl/chip")
local whicapsl = require("../whicapsl-invisible/chip")

local function dimming_chip(user: Object, spec: DimmingChipSpec): Object?
    local ticks = spec.params[1] + spec.params[2] * 0x100
    if ticks == 360 then
        return invisibl.dimming(user, spec)
    elseif ticks == 120 then
        return whicapsl.dimming(user, spec)
    end
    error(string.format("no chip turns its user invisible for %d ticks", ticks))
end

return { dimming_chip = dimming_chip }
