--!strict
-- GroundMan's drill (the original's effect object #0x61, `sub_80E7788`): a
-- sprite on its owner that follows his position, visibility, look and
-- facing. GroundMan's (Param1 1) stays until he takes it off and animates
-- while dimmed; RC Brakr's (his link navi's, navis/groundman: Param1 0,
-- Param2 his action) goes once its owner leaves the action it came with,
-- and animates as `object_updateSpritePaused` does. Visual only, but it
-- holds an effect slot. See docs/engine/chips.md §3.6.25.

local drill = {}

export type State = {
    -- It goes once its owner's action isn't `action` any more (Param1 0);
    -- else it stays until taken off.
    follows_action: boolean,
    action: Action?,
}

local SPRITE = asset.sprite("bn6:ground-drill")

-- `sub_80E77B8`: on its owner, as visible as he is.
local function init(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:copy_visibility(owner)
    me:set_lifecycle("update")
end

-- `sub_80E7816`: follow the owner (his position, visibility, palette,
-- shader, white flash, mosaic, facing and alpha); following an action, go
-- once he has left it. Gone when the battle is over.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local owner = me.related1 :: Object
    local s = me.state :: State
    me.pos = owner.pos
    me:copy_visibility(owner)
    local sprite, look = me.sprite, owner.sprite
    sprite.palette = look.palette
    sprite.color_shader = look.color_shader
    sprite.white = look.white
    sprite.mosaic = look.mosaic
    me.flip = owner.flip
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.alpha = look.alpha
    if s.follows_action and owner:navi_action() ~= s.action then
        me:set_lifecycle("destroy")
    end
end

drill.kind = define.kind {
    id = "bn6:grndman/drill",
    pool = "effect",
    state = { follows_action = "bool", action = "action" },
    -- `sub_80E7788`.
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

-- `sub_80E7896`: a drill on `owner`, staying until taken off
-- (GroundMan's).
function drill.put_on(owner: Object): Object?
    local o = battle.spawn(drill.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80E7896` with Param1 0: a drill on `owner` that goes when he
-- leaves the action he is in (RC Brakr's).
function drill.put_on_for_action(owner: Object): Object?
    local o = drill.put_on(owner)
    if o then
        local s = o.state :: State
        s.follows_action = true
        local a = owner:navi_action()
        if type(a) ~= "table" then
            error("GroundMan's drill follows an action that is no content action")
        end
        s.action = a :: Action
    end
    return o
end

-- `sub_80E78AE`: take the drill off (at its next update).
function drill.take_off(o: Object?)
    if o then
        o:set_lifecycle("destroy")
    end
end

return drill
