--!strict
-- The bullet (the original's attack object #0x12, `sub_80C6946`): the
-- shared bullet (@exelib/bullet/bullet), EXE6's kind, look and variants:
-- the Vulcans' and Spreaders' shot, SpoutCross's charged shot, ColArmy's
-- and ColForce's soldiers'. See docs/engine/shot-chips.md §4.1.

local collision = require("../rules/collision")
local common = require("@exelib/bullet")
local effects = require("../lib/effects")
local regions = require("@exelib/regions")
local statuses = require("../rules/status")

local bullet = {}

-- A bullet variant.
bullet.variant = common.variant

-- A bullet kind (`id`) of `look`.
bullet.make_kind = common.make_kind

-- EXE6's look: the probe's self type (row 0x3D), the burst's (an attack)
-- and what both reach.
local EXE6: BulletLook = { probe = collision.probe, attack = collision.attack, target = collision.hits_navis }

bullet.kind = common.make_kind("bullet", EXE6)

-- `sub_80C6ADA`: a bullet of `variant` from `owner` on panel (x, y) at
-- height `z` (16.16), with `element` and the damage word. Of `kind`, EXE6's
-- by default.
function bullet.fire(owner: Object, x: number, y: number, z: number, variant: BulletVariant, element: number, damage: number, kind: Kind?): Object?
    return common.fire(kind or bullet.kind, owner, x, y, z, variant, element, damage)
end

-- The variants (the original's rows: region, hit modifier, effect,
-- status, sound; rows 0xC, 0xE, 0x10..0x12 show their effect once, at the
-- bullet).
local SPLASH = new.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 }
local HIT = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 0 }
local HIT_BRIGHT = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 1 }
local GLINT = new.effect { sprite = asset.sprite("form-change"), anim = 1, palette = 0 }
local SPARKLE = new.effect { sprite = asset.sprite("colonel-effect"), anim = 3, palette = 0 }
local SPREAD = asset.sound("h-burst")
local SPLASHING = asset.sound("spout-ball")
-- (The fire rows' sound is the one the target squares make as they move.)
local FIRE = asset.sound("target-move")

local function v(region: Region, hit_mod: number, effect: Effect, sound: Sound?, more: { single: boolean?, status: Status? }?): BulletVariant
    local m: { single: boolean?, status: Status? } = more or {}
    return bullet.variant { region = region, hit_mod = hit_mod, effect = effect, sound = sound, single = m.single, status = m.status }
end
local FORK = regions.fork
local ROWS: { [number]: BulletVariant } = {
    [0x00] = v(regions.long, 0, effects.flash, SPREAD),
    [0x01] = v(FORK, 0, effects.flash, SPREAD),
    [0x02] = v(regions.wide, 0, effects.flash, SPREAD),
    [0x03] = v(regions.square, 0, effects.flash, SPREAD),
    [0x04] = v(regions.long, 0, SPLASH, SPLASHING),
    [0x05] = v(FORK, 0, SPLASH, SPLASHING),
    [0x06] = v(regions.wide, 0, SPLASH, SPLASHING),
    [0x07] = v(regions.square, 3, SPLASH, SPLASHING),
    [0x08] = v(regions.long, 0, effects.shell_burst, FIRE),
    [0x09] = v(FORK, 0, effects.shell_burst, FIRE),
    [0x0A] = v(regions.wide, 0, effects.shell_burst, FIRE),
    [0x0B] = v(regions.square, 0, effects.shell_burst, FIRE),
    [0x0C] = v(regions.long, 1, HIT, nil, { single = true }),
    [0x0D] = v(regions.long, 3, SPLASH, SPLASHING),
    [0x0E] = v(regions.long, 1, GLINT, nil, { single = true }),
    [0x0F] = v(regions.square, 1, SPARKLE, SPREAD),
    [0x10] = v(regions.long, 1, HIT_BRIGHT, nil, { single = true }),
    [0x11] = v(regions.long, 1, HIT, nil, { single = true, status = statuses.paralyze_150 }),
    [0x12] = v(regions.long, 1, HIT, nil, { single = true, status = statuses.paralyze_90 }),
}
bullet.variants = {
    -- The Vulcans' (row 0xC) and SuprVulc's (0x10): the panel found and
    -- the one behind it, staggering.
    vulcan = ROWS[0x0C],
    super_vulcan = ROWS[0x10],
    -- The Spreaders' (row 3): the 3x3 around the panel found.
    spreader = ROWS[0x03],
    -- SpoutCross's charged shot (row 0xD).
    spout = ROWS[0x0D],
    -- The patch card puffy's bubble spread (row 7): the 3x3 around the
    -- panel found, with hit modifier 3.
    bubble = ROWS[0x07],
    -- ColArmy's soldiers' (row 0x11) and ColForce's (0x12): staggering,
    -- with their statuses.
    col_army = ROWS[0x11],
    col_force = ROWS[0x12],
}

-- The variant of row `n`, for the callers that still name one by number
-- (the weapons' attack parameters); past the table the game reads on into
-- the routine's code: an error.
function bullet.by_number(n: number): BulletVariant
    local r = ROWS[n]
    if not r then
        error(string.format("bullet row %#x reads past byte_80C68D4", n))
    end
    return r
end

return bullet
