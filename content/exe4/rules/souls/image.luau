--!strict
-- EXE4's soul's image (effect object 0x13, 0x080E361C; spawned by 0x080E37D8: EXE5's code, @exelib/souls/image): the
-- navi MegaMan unites with comes in on a spiral around him for 22 ticks (tinted), blinks for 30 and fades out over 8,
-- on him (0x080E37B2 adds nothing up or on), its first tick the one after it starts (0x080E363C sets its state and
-- returns), its sound 0x157. An effect, it runs while the battle is dimmed as its pool's objects start (0x080032A8's
-- header 0x19). Its look is its soul's own, the soul's form's `image`: its sprite, the soul's navi's (category 8, by
-- the soul). (What it puts on as it comes, by 0x080E36AC's table, WindMan's, FireMan's, NumberMan's and AquaMan's
-- parts, is the souls' own: docs/design/exe4-map.md §18 item 74.)

local common = require("@exelib/souls/image")
local SINE: { number } = require("../math").sine

local image = {}

image.look = common.look

-- Where it is as it starts: the registers the change leaves (0x080EBB20).
local SPAWNED_AT = Vec3.new(0x080E_BB15, 0x40, 0x0200_2610)

-- 0x080E37D8's spawn: the image of the soul `soul` (its form) for `owner`, on its side.
export type Spawn = (owner: Object, soul: FormDef) -> Object?

-- A soul's image, its own kind `id` (each soul's change makes its own, ./change: the original's one effect object is
-- compat's).
function image.make(id: string): Spawn
    local made = common.make {
        id = id,
        pool = "effect",
        sound = asset.sound("soul-image"),
        sine = SINE,
        offset = Vec3.new(0, 0, 0),
        ticks_at_init = false,
        side_only = true,
        runs_while_paused = false,
        priority = 0,
        fade_priority = 2,
    }
    -- (Spawned at the change's registers, garbage its first tick overwrites, which put it off the screen on its first
    -- tick: r1 a code address of the change's, 0x080EBB15 on a US Red Sun console, r2 64, r3 0x02002610. exe4-compat's
    -- kinds.toml skips its place until then.)
    return function(owner: Object, soul: FormDef): Object?
        return made.spawn(owner, soul, SPAWNED_AT)
    end
end

return image
