--!strict
-- The projectile's variants (objects/projectile, the original's
-- `off_80C4C78` rows), written out; a chip's own variant lives with the
-- chip. The bug codes stay the game's numbers.
--
-- Several families fire the first of them. The rest only a navi's stats
-- name: the buster shot's and the charged shot's programs (NaviStats+0x4D
-- and +0x4F), each the variant a shot is on a lucky draw. Each has a key
-- (`shot/...`), which a setup's stats name it by (compat records.toml,
-- `projectile_variants`).

local projectile = require("./projectile")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local variants = {}

local SHOT = { is = collision.attack, hits = collision.hits_navis }
local BLADE = asset.sprite("bn6:sword")

local function v(spec: ProjectileVariant): ProjectileVariant
    return projectile.variant(spec)
end

-- The buster's plain shot.
variants.plain = v { id = "bn6:shot/plain", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain }
-- The cannons' shot: it staggers (hit modifier 3).
variants.cannon = v { id = "bn6:shot/cannon", collision = SHOT, hit_mod = 3, element = "null", spark = sparks.impact }
-- AirShot's shot: a wind shot that pushes.
variants.air = v {
    id = "bn6:shot/air",
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x61,
    element = "null",
    secondary = { "wind" },
    spark = sparks.impact,
}
-- The charged shot.
variants.charged = v { id = "bn6:shot/charged", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- GigaCan's shot: it bursts around what it hits, and over the last two
-- columns when it leaves the field.
variants.gigacan = v { id = "bn6:shot/gigacan", collision = SHOT, hit_mod = 3, element = "null", spark = sparks.blast, bursts = true }
-- CrosOver's MegaMan's shot: it staggers, and climbs a pixel a panel.
variants.climbing = v { id = "bn6:shot/climbing", collision = SHOT, hit_mod = 3, element = "null", spark = sparks.charged, climbs = true }

-- The rest, which only a navi's shot programs name.
local function panel(kind: PanelType): PanelHit
    return { effect = "set_type", left_side = kind, right_side = kind }
end
-- A sword's blade for a shot; it flinches.
variants.blade = v { id = "bn6:shot/blade", collision = SHOT, hit_mod = 1, element = "null", spark = sparks.plain, sprite = BLADE, anim = 1 }
-- A plain shot with the charged shot's spark.
variants.bright = v { id = "bn6:shot/bright", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- A breaking shot.
variants.breaking = v {
    id = "bn6:shot/breaking",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    secondary = { "break" },
    spark = sparks.breaking,
}
-- Shots that change the panel they hit on: cracked, poison, grass, ice.
variants.cracking = v { id = "bn6:shot/cracking", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = { effect = "crack" } }
variants.poisoning = v { id = "bn6:shot/poisoning", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("poison") }
variants.grass = v { id = "bn6:shot/grass", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("grass") }
variants.ice = v { id = "bn6:shot/ice", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("ice") }
-- A shot of the attack type 0x90.
variants.attack_90 = v {
    id = "bn6:shot/attack-90",
    collision = { is = collision.attack_90, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.shot,
}
-- Shots with a status: confusion, blindness, paralysis.
variants.confusing = v { id = "bn6:shot/confusing", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, status = statuses.confuse_480 }
variants.blinding = v { id = "bn6:shot/blinding", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, status = statuses.blind_480 }
variants.paralyzing = v { id = "bn6:shot/paralyzing", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, status = statuses.paralyze_90 }
-- A shot that flinches, and one with the erasing spark.
variants.flinching = v { id = "bn6:shot/flinching", collision = SHOT, hit_mod = 1, element = "null", spark = sparks.plain }
variants.erasing = v { id = "bn6:shot/erasing", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.erase }
-- Charged shots with a status: confusion, paralysis, blindness, a bubble,
-- ice.
variants.charged_confusing = v { id = "bn6:shot/charged-confusing", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = statuses.confuse_480 }
variants.charged_paralyzing = v { id = "bn6:shot/charged-paralyzing", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = statuses.paralyze_90 }
variants.charged_blinding = v { id = "bn6:shot/charged-blinding", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = statuses.blind_480 }
variants.charged_bubbling = v { id = "bn6:shot/charged-bubbling", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = statuses.bubble_150 }
variants.charged_freezing = v { id = "bn6:shot/charged-freezing", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, status = statuses.freeze_150 }
-- Charged shots that change the panel they hit on: cracked, broken,
-- volcano, poison, grass, ice.
variants.charged_cracking = v { id = "bn6:shot/charged-cracking", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = { effect = "crack" } }
variants.charged_panel_breaking = v { id = "bn6:shot/charged-panel-breaking", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = { effect = "break" } }
variants.charged_volcano = v { id = "bn6:shot/charged-volcano", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("volcano") }
variants.charged_poisoning = v { id = "bn6:shot/charged-poisoning", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("poison") }
variants.charged_grass = v { id = "bn6:shot/charged-grass", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("grass") }
variants.charged_ice = v { id = "bn6:shot/charged-ice", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, hit_panel = panel("ice") }
-- Charged shots that lay a road: the way the shot flies for the left
-- side's shots and the other way for the right side's (the routine's own
-- values, 0xC and 0xB, for side 1), and the reverse.
variants.charged_road = v {
    id = "bn6:shot/charged-road",
    collision = SHOT,
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
    hit_panel = { effect = "set_type", left_side = "road_left", right_side = "road_right" },
}
variants.charged_road_back = v {
    id = "bn6:shot/charged-road-back",
    collision = SHOT,
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
    hit_panel = { effect = "set_type", left_side = "road_right", right_side = "road_left" },
}
-- A charged shot of the attack type 0xA0.
variants.charged_attack_a0 = v {
    id = "bn6:shot/charged-attack-a0",
    collision = { is = collision.attack_a0, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}
-- Charged shots with a bug code: the body's programs go (0xFB), the HP
-- bug a level up (0x18 with 1), the HP bug a level up with its mark
-- (0xF4).
variants.charged_uninstalling = v { id = "bn6:shot/charged-uninstalling", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, bug = 0xFB }
variants.charged_hp_bug = v { id = "bn6:shot/charged-hp-bug", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, bug = 0x18, bug_arg = 1 }
variants.charged_hp_bug_marked = v { id = "bn6:shot/charged-hp-bug-marked", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged, bug = 0xF4 }
-- A charged shot that flinches.
variants.charged_flinching = v { id = "bn6:shot/charged-flinching", collision = SHOT, hit_mod = 1, element = "null", spark = sparks.charged }
-- Shots that move what they hit: a panel away (hit modifier 0x61), six
-- panels away (0x49), and a charged one six panels toward the shooter
-- (0x45).
variants.pushing = v { id = "bn6:shot/pushing", collision = SHOT, hit_mod = 0x61, element = "null", spark = sparks.plain }
variants.shoving = v { id = "bn6:shot/shoving", collision = SHOT, hit_mod = 0x49, element = "null", spark = sparks.plain }
variants.charged_pulling = v { id = "bn6:shot/charged-pulling", collision = SHOT, hit_mod = 0x45, element = "null", spark = sparks.charged }

return variants
