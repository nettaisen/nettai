--!strict
-- A Reflector's shield (the original's attack object #0x2B, `sub_80C96A0`):
-- the barrier a navi holds up while it guards. Visual only: the guard is
-- the navi's own status. It follows its owner at the owner's attach point
-- 6, shows while its owner does, and ends with a short fade once the owner
-- clears the slot it was stored in (its first related object). It runs
-- while paused and dimmed, but doesn't animate while dimmed. See chips.md
-- §3.3. The behavior is BN5's too (@exelib/rflectr/shield), made here with
-- BN6's sound and looks.

local common = require("@exelib/rflectr/shield")

local shield = {}

-- A shield's look (`ReflectorShieldLook`, types.d.luau; a row of
-- `byte_80C9664`, by the original's second parameter).
export type Look = ReflectorShieldLook

local look = common.look

-- `byte_80C9664`'s rows: Rflectr1's, the NaviCust Reflect's and Shield's,
-- four no chip or program ported holds up, Rflectr2's and Rflectr3's, and
-- one more nothing ported holds up.
local REFLECTOR, NAVI, SHIELD = asset.sprite("bn6:reflector-shield"), asset.sprite("bn6:megaman-navi"), asset.sprite("bn6:dummy-shield")
local ROWS: { [number]: Look } = {
    [0] = look { sprite = REFLECTOR, anim = 0, fade_anim = 1, fade_ticks = 0xE, palette = 0 },
    [1] = look { sprite = REFLECTOR, anim = 2, fade_anim = 3, fade_ticks = 0xE, palette = 0 },
    [2] = look { sprite = REFLECTOR, anim = 4, fade_anim = 5, fade_ticks = 0xE, palette = 0 },
    [3] = look { sprite = NAVI, anim = 3, fade_anim = 4, fade_ticks = 4, palette = 7 },
    [4] = look { sprite = SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 0 },
    [5] = look { sprite = SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 2 },
    [6] = look { sprite = SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 4 },
    [7] = look { sprite = REFLECTOR, anim = 6, fade_anim = 7, fade_ticks = 0xE, palette = 0 },
    [8] = look { sprite = REFLECTOR, anim = 8, fade_anim = 9, fade_ticks = 0xE, palette = 0 },
    [9] = look { sprite = asset.sprite("bn6:sword"), anim = 0xA, fade_anim = 0xB, fade_ticks = 0xE, palette = 0 },
}
-- The looks by who holds them up: the chips, and the NaviCust programs.
shield.looks = {
    rflectr1 = ROWS[0],
    rflectr2 = ROWS[7],
    rflectr3 = ROWS[8],
    navicust_reflect = ROWS[1],
    navicust_shield = ROWS[2],
}

local made = common.make { id = "bn6:rflectr/shield", sound = asset.sound("bn6:follow-effect") }
shield.kind = made.kind

-- `sub_80C97E0`: a shield of `look` for `owner`, `offset` from it (its
-- attach point, in the spawn position); stored in the owner's first
-- related object, unless the pool is full.
shield.spawn = made.spawn

return shield
