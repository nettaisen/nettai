--!strict
-- ElemTrap's counterattack (the original's effect object #0x2B,
-- `sub_80E35A4`): the shared counterattack (@exelib/elemtrap/strike), BN6's
-- kind and look: its strikes by element, and BN6's panel bursts.

local collision = require("../../rules/collision")
local common = require("@exelib/elemtrap/strike")
local effects = require("../../lib/effects")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local strike = {}

-- The bursts' looks: water for aqua, lightning for elec, leaves for wood
-- (the original's panel-burst rows 0, 2 and 4; fire and the null element
-- take the shells' fiery burst, row 3).
local SPLASH = define.effect { sprite = asset.sprite("bn6:spout-splash"), anim = 0, palette = 0 }
local LIGHTNING = define.effect { sprite = asset.sprite("bn6:lightning"), anim = 0, palette = 0 }
local LEAVES = define.effect { sprite = asset.sprite("bn6:hit-sparks"), anim = 5, palette = 0 }

-- The strike by element (null, fire, aqua, elec, wood): its sound
-- (`byte_80E3714`; none for the null element), its hit (`byte_80E36EC`:
-- the hit spark, hit modifier and status; every one a single panel's, an
-- attack that reaches navis) and the bursts' look (`byte_80E371E`).
local STRIKES: { [number]: ElemTrapStrike } = {
    [0] = { sound = nil, spark = sparks.breaking, hit_mod = 3, status = nil, burst = effects.shell_burst },
    [1] = {
        sound = asset.sound("bn6:target-move"),
        spark = sparks.fire,
        hit_mod = 3,
        status = nil,
        burst = effects.shell_burst,
    },
    [2] = { sound = asset.sound("bn6:spout-ball"), spark = sparks.aqua, hit_mod = 3, status = nil, burst = SPLASH },
    [3] = {
        sound = asset.sound("bn6:beast-over-burst"),
        spark = sparks.elec,
        hit_mod = 1,
        status = statuses.paralyze_90,
        burst = LIGHTNING,
    },
    [4] = { sound = asset.sound("bn6:junk-shot"), spark = sparks.wood, hit_mod = 1, status = statuses.confuse_480, burst = LEAVES },
}
-- BN6's look: the strikes by element, what the hits are and reach, and the
-- bursts' kind.
local BN6: ElemTrapStrikeLook = {
    strikes = STRIKES,
    self_type = collision.attack,
    target = collision.hits_navis,
    bursts = panel_bursts.kind,
}

-- A counterattack kind (`id`) of `look`.
strike.make_kind = common.make_kind

strike.kind = common.make_kind("bn6:elemtrap/strike", BN6)

-- `sub_80E360E`: the counterattack for `user`, of `element`, with the
-- trap's damage word and bonus. Of `kind`, BN6's by default.
function strike.spawn(user: Object, element: number, damage: number, bonus: number, kind: Kind?): Object?
    return common.spawn(kind or strike.kind, user, element, damage, bonus)
end

return strike
