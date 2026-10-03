--!strict
-- Aiming thrown things (@common/trajectory: the game's trajectory helpers
-- and the GBA BIOS routines they call, as the BIOS computes them) over BN5's
-- sine table (rules/math: 0x08005CD0, BN6's values).
--
-- All values are 32-bit integers as the game keeps them: positions and
-- velocities 16.16, angles 256 steps a turn.

return require("@common/trajectory").make(require("../rules/math").sine)
