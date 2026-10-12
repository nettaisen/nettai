--!strict
-- Duo's fists (EXE4's attack object #0x66, 0x080DA220): the shared fist (@exelib/metrknuk/fist: EXE6's and EXE5's
-- MetrKnuk's comes from this code), EXE4's two kinds, one hand's and the other's, which he throws by turns. Each
-- falls fast from high up behind its panel onto it (its sound), hits it (0x080DA40A, 0x15050001: a breaking thrown
-- attack hitting navis on its panel, with the plain spark, hit modifier 3), cracks it and shakes the camera, with a
-- burst (effect row 6) where it is; it goes when the battle ends (0x080DA2E4).
--
-- Its look is his sheet's (08-18) animation 19 less its first parameter (0x080DA28C: the punch's parity, his count's
-- low bit), in palette 0 (10 for a spawner whose own first parameter is 3: the chip's Duo's is its record's, 0).

local collision = require("../../rules/collision")
local common = require("@exelib/metrknuk/fist")
local sparks = require("../../lib/sparks")

local SPRITE = asset.sprite("duo")
local SOUND = asset.sound("justice-one")
local BURST = new.effect { sprite = asset.sprite("burst"), anim = 0, palette = 0 }

local function make(id: string, anim: number)
    return common.make {
        id = id,
        sprite = SPRITE,
        anim = anim,
        sound = SOUND,
        burst = BURST,
        burst_on_fist = true,
        self_type = collision.thrown_break,
        target = collision.hits_navis,
        spark = sparks.plain,
        ends_with_battle = true,
    }
end

return {
    -- An even punch's (the first and the last: its thumb shows) and an odd one's.
    even = make("duo/fist", 19),
    odd = make("duo/other-fist", 18),
}
