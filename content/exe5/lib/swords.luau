--!strict
-- EXE5's swords' parts, for the shared slash (@exelib/swords/slash, EXE6's action 0x13; EXE5's the same code
-- but for a step sword's afterimage and the elemental swords' slash palettes, which these swords haven't): the
-- blade (attachment row 3, as EXE6's: the same look), the slash's effects (EXE5's effects 0x16 to 0x18 are
-- EXE6's: the same rows of 0x080DFC94, by the swords' table 0x080EC028), the swing's sound, and EXE5's
-- collision types, and EXE5's blade animation, which goes by its souls (0x080EC038, `sub_80EBAE8`'s counterpart:
-- `swords.blade_anim`, each sword's `blade_anim`). (Its attachment row, 0x080EC082, `sub_80EBB34`'s counterpart,
-- is row 3 but in Chaos Unison's ProtoSoul and ColonelSoul: Chaos Unison's work, exe5-map.md §15.) EXE5's swing also calls 0x080E9FD2 where EXE6's calls `nullsub_12`: with battle
-- flag 0x40 (`operation_battle()`, never set in a netbattle: exe5-map.md §3) it spawns effect object 0x83
-- on the panel ahead, which isn't ported. And EXE5's sonic boom (attack object #0x58, EXE6's the same number:
-- 0x080D0E64, EXE6's `sub_80CF810`, the same code but for its look): at the floor (EXE6's 16 pixels up), its palette
-- 9 going through (EXE6's 0xB), else by its whole element byte (EXE6's its primary element) from 0x080D0EF4, an EXE5
-- slash.

local antiswrd = require("@exelib/antiswrd/counter")
local attachment = require("../objects/attachment")
local collision = require("../rules/collision")
local slash = require("@exelib/swords/slash")
local sonic_boom = require("@exelib/swords/sonic_boom")
local stepsword = require("./stepsword")
local operation_battle = require("./operation_battle")

local swords = {}

-- 0x080EC038: the blade's animation, which matches its holder's arm: a navi AI's (under 0xD), a player AI's (but
-- 0x17's, 0), or for MegaMan's base AI, his soul's (its form's `blade_anim`: ProtoSoul's, ColonelSoul's and
-- ShadowSoul's arms; 0 in another, and in none).
function swords.blade_anim(me: Object): number
    local kind, ai = me.actor_type, me.ai_index
    if kind == "navi" then
        return if ai < 0x0D then ai else 0
    elseif kind == "player" then
        if ai ~= 0 then
            return if ai == 0x17 then 0 else ai
        end
        local anim: number? = battle.navi(me.alliance).form.blade_anim
        return anim or 0
    end
    return 0
end

local SLASH = asset.sprite("sword-slash")

swords.blade = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
swords.slashes_navis = { is = collision.slash, hits = collision.hits_navis }
swords.effects = {
    -- Sword's, over the panel ahead (effect 0x18).
    slash = new.effect { sprite = SLASH, anim = 2, palette = 0 },
    -- Over a column: WideSwrd's (0x16).
    wide = new.effect { sprite = SLASH, anim = 0, palette = 0 },
    -- Over two panels: LongSwrd's (0x17).
    long = new.effect { sprite = SLASH, anim = 1, palette = 0 },
    -- WideBlde's (0x19) and LongBlde's (0x1A): WideSwrd's and LongSwrd's in another palette.
    wide_blade = new.effect { sprite = SLASH, anim = 0, palette = 5 },
    long_blade = new.effect { sprite = SLASH, anim = 1, palette = 5 },
    -- CustSwrd's big slash over two columns (0x28).
    custom = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 3 },
    -- LifeSrd's big slash over two columns (0x1B).
    big = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 0 },
    -- DrkSword's dark big slash (0x1C).
    dark = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 2 },
    -- Over three panels: FtrSword's (0x2E).
    line = new.effect { sprite = SLASH, anim = 3, palette = 0 },
}
swords.swing = asset.sound("sword-swing")
swords.big_swing = asset.sound("big-sword-swing")
swords.sonic_boom = sonic_boom.make_kind("sonic-boom", {
    sprite = SLASH,
    z = 0,
    piercing_palette = 9,
    palettes = { [0] = 0, 2, 5, 4, 8, 0, 0, 0 },
    element_mask = 0xFF,
    is = collision.slash,
    hits = collision.hits_navis,
})

-- EXE5's sword step (0x080EBD04: by the attack's charge, as its Katanas and MoonBlds take it, lib/stepsword): a
-- charged slash steps two panels ahead, or in ShadowSoul (and by a charge of 2) behind the enemy, turned round.
local STEPS: SlashSteps = {
    steps = function(me: Object): boolean
        return me.charged ~= 0
    end,
    kind = stepsword.kind,
    warp_target = stepsword.behind_enemy,
}

-- A chip's slash (@exelib/swords/slash), its blade EXE5's attachment, its step EXE5's.
function swords.slash(spec: SlashSpec): Action
    return slash.action(spec, attachment.kind, { steps = STEPS })
end

-- AntiSwrd's counter (the original's navi action 0x48: @exelib/antiswrd/counter) with EXE5's blade and booms; its
-- sounds EXE6's. The rules start it by role (`actions.anti_sword_counter`, rules/roles.luau).
swords.anti_sword_counter = antiswrd.action_with {
    id = "antiswrd/counter",
    blade = swords.blade,
    boom = swords.sonic_boom,
    start_sound = asset.sound("log-in"),
    swing_sound = asset.sound("sword-swing"),
    attachment = attachment.kind,
    -- (EXE5's: no invulnerability, nothing restarted at its swings.)
    plain = true,
}

return swords
