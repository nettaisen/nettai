--!strict
-- EXE5's projectile variants (the original's `off_80C4C78` rows), as EXE5's chips need them: for EXE5's projectile
-- (./projectile, EXE6's code) with EXE5's collision types (rules/collision: EXE6's rows without the 0x80 self bit,
-- docs/design/exe5-map.md §15.1).

local collision = require("../../rules/collision")
local projectile = require("../projectile")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local variants = {}

local SHOT = { is = collision.attack, hits = collision.hits_navis }

-- The buster's plain shot (row 0).
variants.plain = projectile.variant { id = "shot/plain", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain }
-- A plain shot with the charged shot's spark (row 3): the auto-battling navi's buster's (rules/auto_battle/buster).
variants.bright = projectile.variant { id = "shot/bright", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- The charged shot (row 6).
variants.charged = projectile.variant { id = "shot/charged", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
-- The cannons' shot (row 1): it staggers (hit modifier 3).
variants.cannon = projectile.variant { id = "shot/cannon", collision = SHOT, hit_mod = 3, element = "null", spark = sparks.impact }

-- The GigaCans' shot (row 0x0C, 0x080C47A8): the blast spark and hit modifier 0x49 (EXE6's 3). It doesn't burst:
-- EXE5's projectile (0x080C48B0) has none of EXE6's row-0x0C bursts, around what it hits or over the last two columns.
variants.gigacan = projectile.variant {
    id = "shot/gigacan",
    collision = SHOT,
    hit_mod = 0x49,
    element = "null",
    spark = sparks.blast,
}

-- AirShot's shot (EXE5's row 4): it pushes (hit modifier 0x61), without EXE6's wind element (EXE5 has none).
variants.air = projectile.variant {
    id = "shot/air",
    collision = { is = collision.pushing, hits = collision.hits_navis },
    hit_mod = 0x61,
    element = "null",
    spark = sparks.impact,
}

-- ---- The shot programs (NaviStats +0x4D, +0x4F) the patch cards give, and Hub Style's spread ----------------------
-- EXE5's projectile (0x080C48B0) reads a row's +3 as the element, +4 as the spark, +8 as the panel type its hit
-- leaves (0xFF: none), +9 as the status, +10 and +11 as the bug code and argument; its hit (0x080C4932) cracks the
-- panel for rows 7 and 0x18 and breaks it for row 0x19, whatever +8 says.

local function panel(kind: PanelType): PanelHit
    return { effect = "set_type", left_side = kind, right_side = kind }
end
local function charged(id: string, extra: { [string]: any }): ProjectileVariant
    local spec: { [string]: any } = { id = id, collision = SHOT, hit_mod = 0, element = "null", spark = sparks.charged }
    for k, v in extra do
        spec[k] = v
    end
    return projectile.variant(spec :: any)
end

-- Row 5: a breaking shot (collision row 6, the breaking spark; EXE5 has no secondary elements).
variants.breaking = projectile.variant {
    id = "shot/breaking",
    collision = { is = collision.breaking, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.breaking,
}
-- Row 7: a shot that cracks the panel it hits (CrakBust).
variants.cracking = projectile.variant {
    id = "shot/cracking",
    collision = SHOT,
    hit_mod = 0,
    element = "null",
    spark = sparks.plain,
    hit_panel = { effect = "crack" },
}
-- Rows 8, 0x0D, 0x0E, 0x14: shots that turn the panel they hit poison (SwmpBust), grass, ice, sea (WtrBust).
variants.poisoning = projectile.variant {
    id = "shot/poisoning",
    collision = SHOT,
    hit_mod = 0,
    element = "null",
    spark = sparks.plain,
    hit_panel = panel("poison"),
}
variants.grass = projectile.variant { id = "shot/grass", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("grass") }
variants.ice = projectile.variant { id = "shot/ice", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("ice") }
variants.sea = projectile.variant { id = "shot/sea", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.plain, hit_panel = panel("sea") }
-- Row 9: the chip destruction shot, with the chip destruction hit spark (CSchpdst).
variants.chip_destruction = projectile.variant {
    id = "shot/chip-destruction",
    collision = { is = collision.chip_destruction, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.chip_destruction,
}
-- Row 0x12: Hub Style's spread shot (0x0800F522), the erase beams' spark.
variants.hub = projectile.variant { id = "shot/hub", collision = SHOT, hit_mod = 0, element = "null", spark = sparks.erase }
-- Rows 0x15 to 0x17: charged shots that confuse, paralyze, blind.
variants.charged_confusing = charged("shot/charged-confusing", { status = statuses.confuse_480 })
variants.charged_paralyzing = charged("shot/charged-paralyzing", { status = statuses.paralyze_90 })
variants.charged_blinding = charged("shot/charged-blinding", { status = statuses.blind_480 })
-- Rows 0x18 to 0x1B: charged shots that crack the panel they hit, break it (CShole), turn it lava, poison.
variants.charged_cracking = charged("shot/charged-cracking", { hit_panel = { effect = "crack" } })
variants.charged_panel_breaking = charged("shot/charged-panel-breaking", { hit_panel = { effect = "break" } })
variants.charged_lava = charged("shot/charged-lava", { hit_panel = panel("lava") })
variants.charged_poisoning = charged("shot/charged-poisoning", { hit_panel = panel("poison") })
-- Row 0x1C: a charged shot of the attack type 0xA0 (collision row 0x47: CSbrrdlt, it takes barriers away).
variants.charged_attack_a0 = charged("shot/charged-attack-a0", { collision = { is = collision.attack_a0, hits = collision.hits_navis } })
-- Rows 0x1D, 0x1E: charged shots with a bug code: the programs go (0xFB, CSreset), the HP bug (0x18 with 0x11).
variants.charged_uninstalling = charged("shot/charged-uninstalling", { bug = 0xFB })
variants.charged_hp_bug = charged("shot/charged-hp-bug", { bug = 0x18, bug_arg = 0x11 })

return variants
