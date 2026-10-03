--!strict
-- BN5's swords' parts, for the shared slash (@common/swords/slash, BN6's action 0x13; BN5's the same code
-- but for a step sword's afterimage and the elemental swords' slash palettes, which these swords haven't): the
-- blade (attachment row 3, as BN6's: the same look), the slash's effects (BN5's effects 0x16 to 0x18 are
-- BN6's: the same rows of 0x080DFC94, by the swords' table 0x080EC028), the swing's sound, and BN5's
-- collision types, and BN5's blade animation, which goes by its souls (0x080EC038, `sub_80EBAE8`'s counterpart:
-- `swords.blade_anim`, each sword's `blade_anim`). (Its attachment row, 0x080EC082, `sub_80EBB34`'s counterpart,
-- is row 3 but in Chaos Unison's ProtoSoul and ColonelSoul: Chaos Unison's work, bn5-map.md §15.) BN5's swing also calls 0x080E9FD2 where BN6's calls `nullsub_12`: with battle
-- flag 0x40 (`battle.per_player_gauges()`, never set in a netbattle: bn5-map.md §3) it spawns effect object 0x83
-- on the panel ahead, which isn't ported. And BN5's sonic boom (attack object #0x58, BN6's the same number:
-- 0x080D0E64, BN6's `sub_80CF810`, the same code but for its look): at the floor (BN6's 16 pixels up), its palette
-- 9 going through (BN6's 0xB), else by its whole element byte (BN6's its primary element) from 0x080D0EF4, a BN5
-- slash.

local antiswrd = require("@common/antiswrd/counter")
local attachment = require("../objects/attachment/attachment")
local collision = require("../rules/collision")
local slash = require("@common/swords/slash")
local sonic_boom = require("@common/swords/sonic_boom")
local stepsword = require("./stepsword")

local swords = {}

-- The blade's animations of MegaMan's souls (0x080EC038): ProtoSoul's, ColonelSoul's and ShadowSoul's arms.
local SOUL_BLADE_ANIMS: { [number]: number } = { [1] = 13, [7] = 14, [8] = 15 }

-- 0x080EC038: the blade's animation, which matches its holder's arm: a navi AI's (under 0xD), a player AI's (but
-- 0x17's, 0), or for MegaMan's base AI, his soul's (0 in none).
function swords.blade_anim(me: Object): number
    local kind, ai = me.actor_type, me.ai_index
    if kind == "navi" then
        return if ai < 0x0D then ai else 0
    elseif kind == "player" then
        if ai ~= 0 then
            return if ai == 0x17 then 0 else ai
        end
        local soul = battle.navi(me.alliance).form.soul
        return if soul then SOUL_BLADE_ANIMS[soul.number] or 0 else 0
    end
    return 0
end

local SLASH = asset.sprite("bn5:sword-slash")

swords.blade = attachment.look { sprite = asset.sprite("bn5:sword"), palette = 0 }
swords.slashes_navis = { is = collision.slash, hits = collision.hits_navis }
swords.effects = {
    -- Sword's, over the panel ahead (effect 0x18).
    slash = define.effect { sprite = SLASH, anim = 2, palette = 0 },
    -- Over a column: WideSwrd's (0x16).
    wide = define.effect { sprite = SLASH, anim = 0, palette = 0 },
    -- Over two panels: LongSwrd's (0x17).
    long = define.effect { sprite = SLASH, anim = 1, palette = 0 },
    -- WideBlde's (0x19) and LongBlde's (0x1A): WideSwrd's and LongSwrd's in another palette.
    wide_blade = define.effect { sprite = SLASH, anim = 0, palette = 5 },
    long_blade = define.effect { sprite = SLASH, anim = 1, palette = 5 },
    -- CustSwrd's big slash over two columns (0x28).
    custom = define.effect { sprite = asset.sprite("bn5:big-slash"), anim = 0, palette = 3 },
    -- LifeSrd's big slash over two columns (0x1B).
    big = define.effect { sprite = asset.sprite("bn5:big-slash"), anim = 0, palette = 0 },
    -- DrkSword's dark big slash (0x1C).
    dark = define.effect { sprite = asset.sprite("bn5:big-slash"), anim = 0, palette = 2 },
    -- Over three panels: FtrSword's (0x2E).
    line = define.effect { sprite = SLASH, anim = 3, palette = 0 },
}
swords.swing = asset.sound("bn5:sword-swing")
swords.big_swing = asset.sound("bn5:big-sword-swing")
swords.sonic_boom = sonic_boom.make_kind("bn5:sonic-boom", {
    sprite = SLASH,
    z = 0,
    piercing_palette = 9,
    palettes = { [0] = 0, 2, 5, 4, 8, 0, 0, 0 },
    element_mask = 0xFF,
    is = collision.slash,
    hits = collision.hits_navis,
})

-- BN5's sword step (0x080EBD04: by the attack's charge, as its Katanas and MoonBlds take it, lib/stepsword): a
-- charged slash steps two panels ahead, or in ShadowSoul (and by a charge of 2) behind the enemy, turned round.
local STEPS: SlashSteps = {
    steps = function(me: Object): boolean
        return me.charged ~= 0
    end,
    kind = stepsword.kind,
    warp_target = stepsword.behind_enemy,
}

-- A chip's slash (@common/swords/slash), its blade BN5's attachment, its step BN5's.
function swords.slash(spec: SlashSpec): Action
    return slash.action(spec, attachment.kind, STEPS)
end

-- AntiSwrd's counter (the original's navi action 0x48: @common/antiswrd/counter) with BN5's blade and booms; its
-- sounds BN6's. The ruleset starts it by role (`actions.anti_sword_counter`, rules/roles.luau).
swords.anti_sword_counter = antiswrd.action_with {
    id = "bn5:antiswrd/counter",
    blade = swords.blade,
    boom = swords.sonic_boom,
    start_sound = asset.sound("bn6:log-in"),
    swing_sound = asset.sound("bn6:sword-swing"),
    attachment = attachment.kind,
}

return swords
