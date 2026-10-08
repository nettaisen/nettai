--!strict
-- What SlashCross's charged slash (./charge, the original's action 0x41)
-- makes of a sword: the wave it sends (./sword_wave), the blade the navi
-- raises (none: it swings its own arm), the swing's sound, a second,
-- hidden wave (CrosSwrd's: the plain sword's), a second slash after it
-- (DblDream's), and the lock-on mode the Beast Out rush takes for it.
--
-- A sword states its own in its slash's spec (`charged = slashes.slash {
-- ... }`, lib/swords/slash.luau; MoonBld in its action's), which the charge
-- reads of the attack's chip. (The original has one table by the charged
-- chip's subtype for each of these: the wave's rows, the blade
-- `byte_80EBB64`, the lock-on mode `byte_80EB028`, and tests of the
-- subtype for the rest.)

local sword_wave = require("./sword_wave")

local slashes = {}

-- A charged slash.
function slashes.slash(s: ChargedSlash): ChargedSlash
    return new.record("charged-slash", s)
end

-- A sword wave of the rows' usual make (./sword_wave).
slashes.wave = sword_wave.wave

return slashes
