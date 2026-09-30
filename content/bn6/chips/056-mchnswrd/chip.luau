--!strict
-- Registration by number, until chips are handles
-- (docs/design/content-model-v2.md §12, step 3b): the machine swords'
-- records name the original's action 0x49 and their subtype, and
-- registration by number runs this module for them (and for the stun
-- strike the navi's own request 0x80000 starts, which nothing in the game
-- sets). It runs the chip's own strike (chips/mchnswrd, chips/elemswrd,
-- chips/assnswrd), picked by the subtype. It goes when the chips'
-- definitions replace their records.

local strike = require("../../lib/swords/strike")
local mchnswrd = require("../mchnswrd/chip")
local elemswrd = require("../elemswrd/chip")
local assnswrd = require("../assnswrd/chip")

local legacy = {
    state = strike.STATE,
}

local BY_SUBTYPE: { [number]: Action } = {
    [0] = mchnswrd.action,
    [1] = elemswrd.action,
    [2] = assnswrd.action,
}

function legacy.update(me: Object, s: any)
    local a = BY_SUBTYPE[me.variant]
    if not a then
        error(string.format("machine sword subtype %#x has no chip", me.variant))
    end
    a.update(me, s)
end

return legacy
