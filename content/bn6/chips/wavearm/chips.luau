--!strict
-- WaveArm1, WaveArm2 and WaveArm3: a shock wave (wave.luau) rolling from
-- the panel ahead that steers toward an enemy's row (strike.luau).
--
-- Their records stay the pack's for now (PwrWave's Program Advance recipes
-- name them by number): these are their actions, which registration by
-- number runs (chips/05c-wavearm1, docs/design/content-model-v2.md §12).

local strike = require("./strike")
local wave = require("./wave")

return {
    { action = strike.action { id = "wavearm1/action", wave = wave.variants[0xC] } },
    { action = strike.action { id = "wavearm2/action", wave = wave.variants[0xD] } },
    { action = strike.action { id = "wavearm3/action", wave = wave.variants[0xE] } },
}
