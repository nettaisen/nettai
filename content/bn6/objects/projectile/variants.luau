--!strict
-- The projectile's variants several families fire (objects/projectile,
-- the original's `off_80C4C78` rows), written out; a chip's own variant
-- lives with the chip. The statuses and bug codes stay the game's numbers
-- until statuses are definitions.
--
-- `by_number`: the variants by the original's row number, for what still
-- names one by number: the navi stats' NaviCust shots (the buster's and
-- the charged shot's program bytes, NaviStats+0x4D and +0x4F), which the
-- weapons' setups turn into variants until the stats carry them
-- (docs/design/content-model-v2.md §5.3, §7.2).

local projectile = require("./projectile")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local variants = {}

local SHOT = { is = collision.attack, hits = collision.hits_navis }
local BLADE = asset.sprite("sword")

local function v(spec: ProjectileVariant): ProjectileVariant
    return projectile.variant(spec)
end

-- The buster's plain shot (row 0).
variants.plain = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain }
-- The cannons' shot (row 1): it staggers (hit modifier 3).
variants.cannon = v { collision = SHOT, hit_mod = 3, element = "null", spark = sparks.impact }
-- AirShot's shot (row 4): a wind shot that pushes.
variants.air = v {
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x61,
    element = "null",
    secondary = { "wind" },
    spark = sparks.impact,
}
-- The charged shot (row 6).
variants.charged = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- GigaCan's shot (row 0xC): it bursts around what it hits, and over the
-- last two columns when it leaves the field.
variants.gigacan = v { collision = SHOT, hit_mod = 3, element = "null", spark = sparks.blast, bursts = true }

-- The rest of the rows, which only the NaviCust's programs name.
local function panel(kind: PanelType): PanelHit
    return { effect = "set_type", left_side = kind, right_side = kind }
end
local ROWS: { [number]: ProjectileVariant } = {
    [0x00] = variants.plain,
    [0x01] = variants.cannon,
    [0x02] = v { collision = SHOT, hit_mod = 1, element = "null", spark = sparks.plain, sprite = BLADE, anim = 1 },
    [0x03] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged },
    [0x04] = variants.air,
    [0x05] = v {
        collision = { is = collision.breaking, hits = collision.hits_navis },
        hit_mod = 0,
        element = "null",
        secondary = { "break" },
        spark = sparks.breaking,
    },
    [0x06] = variants.charged,
    [0x07] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = { effect = "crack" } },
    [0x08] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("poison") },
    [0x09] = v {
        collision = { is = collision.attack_90, hits = collision.hits_navis },
        hit_mod = 0,
        element = "null",
        spark = sparks.shot,
    },
    [0x0A] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, status = 0x20 },
    [0x0B] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, status = 0x30 },
    [0x0C] = variants.gigacan,
    [0x0D] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("grass") },
    [0x0E] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("ice") },
    [0x0F] = v { collision = SHOT, hit_mod = 1, element = "null", spark = sparks.plain },
    [0x10] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.erase },
    [0x11] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, status = 0x10 },
    [0x12] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = 0x20 },
    [0x13] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = 0x10 },
    [0x14] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = 0x30 },
    [0x15] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = { effect = "crack" } },
    [0x16] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = { effect = "break" } },
    [0x17] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("volcano") },
    [0x18] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("poison") },
    [0x19] = v {
        collision = { is = collision.attack_a0, hits = collision.hits_navis },
        hit_mod = 0,
        element = "null",
        spark = sparks.charged,
    },
    [0x1A] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, bug = 0xFB },
    [0x1B] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, bug = 0x18, bug_arg = 1 },
    [0x1C] = v { collision = SHOT, hit_mod = 1, element = "null", spark = sparks.charged },
    [0x1D] = v { collision = SHOT, hit_mod = 3, element = "null", spark = sparks.charged, climbs = true },
    [0x1E] = v { collision = SHOT, hit_mod = 0x61, element = "null", spark = sparks.plain },
    [0x1F] = v { collision = SHOT, hit_mod = 0x49, element = "null", spark = sparks.plain },
    [0x20] = v { collision = SHOT, hit_mod = 0x45, element = "null", spark = sparks.charged },
    [0x21] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = 0x60 },
    -- A road the way the shot flies for the left side's shots, and the
    -- other way for the right side's (the routine's own values, 0xC and
    -- 0xB, for side 1).
    [0x22] = v {
        collision = SHOT,
        hit_mod = 0,
        element = "null",
        spark = sparks.charged,
        hit_panel = { effect = "set_type", left_side = "road_left", right_side = "road_right" },
    },
    [0x23] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, bug = 0xF4 },
    [0x24] = v {
        collision = SHOT,
        hit_mod = 0,
        element = "null",
        spark = sparks.charged,
        hit_panel = { effect = "set_type", left_side = "road_right", right_side = "road_left" },
    },
    [0x25] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("grass") },
    [0x26] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = 0x50 },
    [0x27] = v { collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("ice") },
}

-- The variant of row `n` (a row past the table reads on into the
-- routine's code in the game: an error).
function variants.by_number(n: number): ProjectileVariant
    local r = ROWS[n]
    if not r then
        error(string.format("projectile row %#x reads past off_80C4C78", n))
    end
    return r
end

return variants
