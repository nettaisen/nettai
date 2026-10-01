--!strict
-- PwrWave1, PwrWave2 and PwrWave3 (WaveArm's Program Advances): WaveArm's
-- strike (chips/wavearm/strike.luau) with a straight wave, and two more
-- from the navi's own column along the rows above and below.
--
-- Their records stay the pack's for now (Program Advances): these are
-- their actions, which registration by number runs (chips/05c-wavearm1,
-- docs/design/content-model-v2.md §12).

local strike = require("../wavearm/strike")
local wave = require("../wavearm/wave")

return {
    { action = strike.action { id = "pwrwave1/action", wave = wave.variants.pwrwave, three_rows = true } },
    { action = strike.action { id = "pwrwave2/action", wave = wave.variants.pwrwave, three_rows = true } },
    { action = strike.action { id = "pwrwave3/action", wave = wave.variants.pwrwave, three_rows = true } },
}
