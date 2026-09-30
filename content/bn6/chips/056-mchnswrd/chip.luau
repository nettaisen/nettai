--!strict
-- Registration by number (docs/design/content-model-v2.md §5.2, "As
-- built"): action 0x49 runs this module, which runs the machine sword's
-- own strike (chips/mchnswrd, chips/elemswrd, chips/assnswrd) picked by the
-- subtype. The chips are definitions and run their strikes without it;
-- what still reaches it is the stun strike the ruleset starts by number
-- (the navi's request 0x80000, which nothing in the game sets; with the
-- variant the attack holds) and the test content's StunBld. It goes when
-- the stun strike is a role (phase C).

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
