--!strict
-- Registration by number (docs/design/content-model-v2.md §12): the
-- records that name the original's action 0x43 (AuraHed1-3 and StreamHd,
-- kept as records: the ruleset knows them by number as the aura chips,
-- and StreamHd is a Program Advance) run this module, which runs the
-- chip's own action (chips/aurahed): StreamHd's for the stream (the
-- record's subtype 1), else the AuraHed of the head's palette (its first
-- parameter). It goes when the records are definitions (step 5, and an
-- aura trait for the ruleset, phase C).

local action = require("../aurahed/action")
local aurahed = require("../aurahed/chips")

local by_number = {
    state = action.STATE,
}

-- The AuraHeds by palette.
local BY_PALETTE: { [number]: Action } = {
    [0] = aurahed.aurahed1.action,
    [1] = aurahed.aurahed2.action,
    [2] = aurahed.aurahed3.action,
}

function by_number.update(me: Object, s: any)
    -- (The original streams for any subtype but 0.)
    if me.variant ~= 0 then
        aurahed.streamhd.action.update(me, s)
        return
    end
    local palette = me:attack_param(1)
    local a = BY_PALETTE[palette]
    if not a then
        error(string.format("AuraHed palette %d has no chip", palette))
    end
    a.update(me, s)
end

return by_number
