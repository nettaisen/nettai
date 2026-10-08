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
local parts = require("@exelib/swords/parts")
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

-- The blade the swords hold (the attachment table's row 3) and the swings' sounds, which they share. (Each sword
-- states the slash drawn over the panels it hits.)
swords.blade = attachment.look { sprite = asset.sprite("sword"), palette = 0 }

-- 0x080EC082: the blade a sword's holder takes (the attachment row): the swords' (row 3), but MegaMan's (a player of
-- AI index 0) while a Chaos Unison charge is armed (AIData+0x12, the rules' own B charge: rules/souls/chaos_charge)
-- takes his soul's Chaos Unison blade where it states one (`chaos.blade`: ProtoSoul's row 10, ColonelSoul's 0x32).
function swords.held_blade(me: Object): AttachmentLook
    if me.actor_type == "player" and me.ai_index == 0 and me.b_charge_time ~= nil then
        local chaos = (battle.navi(me.alliance).form :: any).chaos
        if chaos ~= nil and chaos.blade ~= nil then
            return chaos.blade :: AttachmentLook
        end
    end
    return swords.blade
end

-- How a sword's holder takes its blade in the related slot (EXE5's slash, 0x080EBE1C): the held blade, in the
-- blade's palette (@exelib/swords/parts, as EXE6's).
local function hold_in_related(me: Object, _blade: AttachmentLook, anim: number)
    parts.hold(attachment.kind, me, swords.held_blade(me), parts.blade_palette(me), anim)
end
swords.slashes_navis = { is = collision.slash, hits = collision.hits_navis }
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
    return slash.action(spec, attachment.kind, { steps = STEPS, hold = hold_in_related })
end

-- AntiSwrd's counter (the original's navi action 0x48: @exelib/antiswrd/counter) with EXE5's blade and booms; its
-- sounds EXE6's. The rules start it by role (`actions.anti_sword_counter`, rules/roles.luau). It holds the held blade
-- in EXE5's blade animation (EXE5's action 0x2D, 0x080EDB26 and 0x080EDB2C: 0x080EC082 and 0x080EC038).
swords.anti_sword_counter = antiswrd.action_with {
    id = "antiswrd/counter",
    blade = swords.blade,
    hold = function(me: Object, _blade: AttachmentLook)
        parts.hold(attachment.kind, me, swords.held_blade(me), 0, swords.blade_anim(me))
    end,
    boom = swords.sonic_boom,
    start_sound = asset.sound("log-in"),
    swing_sound = asset.sound("sword-swing"),
    attachment = attachment.kind,
    -- (EXE5's: no invulnerability, nothing restarted at its swings.)
    plain = true,
}

return swords
