--!strict
-- JudgeMan's whip (the original's effect object #0x5A, `sub_80E70C8`): his
-- sprite's whip animation on him. JudgeMan's own (Param1 1) animates while
-- dimmed and stays until he takes it off; the navi AI's (Param1 0) goes
-- once its owner leaves the action it came with, and waits a dimming out.
-- See docs/engine/chips.md §3.6.29.

local whip = {}

export type State = {
    -- It goes once its owner's action isn't `action` any more, and waits
    -- a dimming out (Param1 0); else it stays until taken off.
    follows_action: boolean,
    action: Action?,
}

local SPRITE = asset.sprite("bn6:judgeman")
local ANIM_RAISE, ANIM_HOLD = 8, 9

-- `sub_80E70F8`: on its owner.
local function init(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = ANIM_RAISE
    me.anim_loaded = ANIM_RAISE
    sprite:set_animation(ANIM_RAISE)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E717A`: 15 ticks raised, then held.
local function swing(me: Object)
    local a = me.action
    if a == 8 then
        return
    end
    if a ~= 4 then
        me.action = 4
        me.timer = 0xF
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.anim = ANIM_HOLD
        me.action = 8
    end
end

-- `sub_80E7142`: on its owner; following an action, it goes once he has
-- left it and holds still while dimmed. Gone when the battle is over.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local owner = me.related1 :: Object
    me.pos = owner.pos
    local s = me.state :: State
    if s.follows_action then
        if owner:navi_action() ~= s.action then
            me:set_lifecycle("destroy")
            return
        end
        if battle.dimmed() then
            return
        end
    end
    swing(me)
end

whip.kind = define.kind {
    id = "bn6:judgeman/whip",
    pool = "effect",
    state = { follows_action = "bool", action = "action" },
    -- `sub_80E70C8`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        if (me.state :: State).follows_action then
            me:update_sprite_while_paused()
        else
            me:update_sprite_while_dimmed()
        end
    end,
}

-- `sub_80E71A0`: a whip on `owner`, staying until taken off (JudgeMan's).
function whip.raise(owner: Object): Object?
    local o = battle.spawn(whip.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80E71A0` with Param1 0 (the navi AI's): a whip on `owner` that
-- goes when he leaves the action he is in. [unverified: no navi chip uses
-- it]
function whip.raise_for_action(owner: Object): Object?
    local o = whip.raise(owner)
    if o then
        local s = o.state :: State
        s.follows_action = true
        local a = owner:navi_action()
        if type(a) ~= "table" then
            error("JudgeMan's whip follows an action that is no content action")
        end
        s.action = a :: Action
    end
    return o
end

-- `sub_80E71B8`: take the whip off (at its next update).
function whip.take_off(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

return whip
