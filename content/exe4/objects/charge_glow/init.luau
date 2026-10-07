--!strict
-- EXE4's charge glow (effect object 5, 0x080E215C): a navi's charge brings it (`effects.charge_glow = "with_charge"`:
-- the navi's tail spawns the role `kinds.charge_glow` while its link, AIData+0x58, is empty and its charge has a level,
-- 0x0800BD88; the navi its first related). It glows for the source charging as it came, and goes (forgetting the
-- navi's link) once the navi charges another source or its charge's level drops to none. Its animation is the level's:
-- 0 charging, 2 full (EXE4's levels 1 and 3, less one). Every console hears it come (0x71) and the charge fill (0x72).
-- It sits on its navi: on its position, offset by its navi's row of 0x080E2274 (by NaviStats +0x23), MegaMan's (row 0)
-- by none; EXE4's content has no other navi.

local glow = {}

local B_GLOW = asset.sprite("charge-glow")
local A_GLOW = asset.sprite("charge-glow-a")
local CHARGING = asset.sound("buster-charge")
local CHARGED = asset.sound("buster-charged")

export type State = {
    -- The charge source it glows for (its +6: 1 A, 2 B).
    source: number,
    -- Shown with its navi (its +0x0C): dimming hides it.
    shown: boolean,
}

-- 0x080E2264: the navi forgets it (the link slot it was given, AIData+0x58), and it goes.
local function finish(me: Object, navi: Object)
    navi.charge_glow = nil
    me:free()
end

-- 0x080E21D2: seen by whoever sees its navi's side, while shown; gone once its source isn't the navi's or the charge
-- has no level; else the level's animation, on its navi.
local function tick(me: Object)
    local navi = me.related1 :: Object
    local state = me.state :: State
    me.visible = true
    me:hide_from_blind()
    if not state.shown then
        me.visible = false
    end
    if navi.charge_source ~= state.source or navi.charge_level == 0 then
        return finish(me, navi)
    end
    local before = me.anim
    local anim = if navi.charge_level == 2 then 2 else 0
    me.anim = anim
    if anim ~= before and anim == 2 then
        battle.play_sound(CHARGED)
    end
    me.pos = navi.pos
    me:update_sprite()
end

-- 0x080E217C: the source charging now (none: it goes at once); its sprite by it (14-08 for B's, 14-15 for A's), no
-- shadow, animation 0; the charging sound; its first update at once.
local function init(me: Object)
    local navi = me.related1 :: Object
    local state = me.state :: State
    local source = navi.charge_source
    state.source = source
    if source == 0 then
        return finish(me, navi)
    end
    local sprite = me.sprite
    sprite:load(if source == 2 then B_GLOW else A_GLOW)
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    me.visible = true
    battle.play_sound(CHARGING)
    me:set_lifecycle("update")
    tick(me)
end

glow.kind = new.kind {
    id = "charge-glow",
    pool = "effect",
    state = { source = "u8", shown = "bool" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

return glow
