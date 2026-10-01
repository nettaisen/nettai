--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x31 runs
-- this module, which runs the chip's own strike (chips/wavearm,
-- chips/pwrwave) picked by the subtype as `sub_80ED64C` branches. The
-- records that reach it are WaveArm1 to 3 (PwrWave's recipes name them by
-- number) and PwrWave1 to 3 (Program Advances). Within a subtype the
-- records differ only in their damage, which the attack holds, and their
-- waves alike (WaveArm1 to 3's are `byte_80C6B00`'s identical rows 0xC to
-- 0xE), so the first chip's strike runs for each. It goes when the
-- records are definitions (step 5, phase C).

local strike = require("../wavearm/strike")
local wavearm = require("../wavearm/chips")
local pwrwave = require("../pwrwave/chips")

local by_number = {
    state = strike.STATE,
}

-- The strikes by subtype.
local BY_SUBTYPE: { [number]: Action } = {
    [0] = wavearm[1].action,
    [1] = pwrwave[1].action,
}

function by_number.update(me: Object, s: any)
    local subtype = me.variant
    local a = BY_SUBTYPE[subtype]
    if not a then
        error(string.format("WaveArm subtype %#x has no strike (sub_80ED64C)", subtype))
    end
    a.update(me, s)
end

return by_number
