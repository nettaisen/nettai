--!strict
-- EXE4's swords' parts, for the shared slash (@exelib/swords/slash; EXE4's action 0x0A, 0x080EB51E, EXE6's and EXE5's
-- code but for its `SlashGame`): the blade (attachment row 3, 0c-00 in palette 0), held in the related slot as EXE4's
-- own attachment, its animation the navi's number (NaviStats +0x23, MegaMan's 0), in ProtoSoul (soul 7, 0x080EB628)
-- 13; its hits' modifier the chip's, in ProtoSoul 0x080EB7FE's by the variant (0x080EB6E8: 1 for every sword's,
-- variants 0 to 11); a swing that restarts no form overlay and a recovery that leaves the moving flag. Each chip
-- composes its slash from its variant's rows (`swords.slash { ... }`): the hit (0x080EB7B0's region word: the region,
-- the target and self collision rows), its hit modifier (0x080EB7F1), the slash's effect (0x080EB7E4, a row of the
-- effect table 0x080E1698: the chip's own, `new.effect` of the slash sprites 0c-14 and 0c-15), the swing's sound (0x8E;
-- variants 5 and 11 0xB4), the swings (variant 11's two, 0x080EB5E4), a second hit on the same panel (variant 10's:
-- 0x0705FF01); a charged slash steps (0x080EB548, below).

local attachment = require("../objects/attachment")
local slash = require("@exelib/swords/slash")

local swords = {}

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0); ProtoSoul's blade (its animation 13)
-- and its swords' hit modifier (0x080EB7FE's rows of the swords' variants).
local NAVI_NUMBER = 0
local PROTOSOUL_BLADE, PROTOSOUL_HIT_MOD = 13, 1

-- In ProtoSoul (NaviStats +0x24 7).
local function in_protosoul(me: Object): boolean
    return battle.navi(me.alliance).form.id == "protosoul"
end

-- The blade a navi holds (attachment row 3, 0x080CD524: 0c-00 in palette 0).
swords.blade = attachment.look { sprite = asset.sprite("sword"), palette = 0 }

-- The swings' sounds (0x080EB612).
swords.swing = asset.sound("sword-swing")
swords.big_swing = asset.sound("big-sword-swing")

-- EXE4's sword step (0x080EB548, as EXE5's grew from it): a charged slash (AIAttackVars +4: ProtoSoul's charged
-- Swords) steps two panels ahead, if the navi can move (`object_canMove`, 0x0800AD2A: it holds its panel, setting no
-- moving flag); with no panel there it doesn't swing. EXE4 has no step behind the enemy.
local STEPS: SlashSteps = {
    steps = function(me: Object): boolean
        return me.charged ~= 0
    end,
    kind = function(me: Object, _set_off: (Object) -> boolean): ("step" | "warp")?
        if me.charged == 0 or not me:can_move() then
            return nil
        end
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me:reserve_panel(me.panel_x, me.panel_y)
        return "step"
    end,
    warp_target = function(_me: Object): (number?, number?)
        return nil, nil
    end,
}

-- The blade's animation as a sword holds it (the slash's 0x080EB628, SonicBom's 0x080ED8A0, Z Saver's 0x080EE820): in
-- ProtoSoul (NaviStats +0x24 7) 13, else the navi's number.
function swords.blade_anim(me: Object): number
    return if in_protosoul(me) then PROTOSOUL_BLADE else NAVI_NUMBER
end

local GAME: SlashGame = {
    steps = STEPS,
    -- 0x080EB624.
    blade_anim = swords.blade_anim,
    -- 0x080EB590, 0x080EB676: the afterimages of the navi, of its sprite in its form (0x0800B90A).
    navi_afterimage = function(me: Object): SpriteAsset
        return battle.navi(me.alliance).form.sprite
    end,
    -- 0x080EB6E8: the hit modifier by the variant, ProtoSoul's from its own row.
    hit_mod = function(me: Object, chip_hit_mod: number): number
        return if in_protosoul(me) then PROTOSOUL_HIT_MOD else chip_hit_mod
    end,
    hold = function(me: Object, blade: AttachmentLook, anim: number)
        attachment.attach(me, blade, "related", anim)
    end,
    refreshes_form_overlay = false,
    recovery_clears_moving = false,
}

-- A chip's slash.
function swords.slash(spec: SlashSpec): Action
    return slash.action(spec, attachment.kind, GAME)
end

return swords
