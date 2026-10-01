--!strict
-- DustMan's DustBrk looks (the original's effect objects #0x72,
-- `sub_80E8794`, and #0x71, `sub_80E8688`): a dust cloud, and a second
-- copy of his own sprite, riding on him in his look (his visibility,
-- palette, shading, flash, mosaic, facing and blending) while he stays in
-- the action that put them on (the original names that action by number,
-- their Param2 and Param1). Visual only, but each holds an effect slot.
-- See docs/engine/standard-chips.md, "Action 0x0A".

local clouds = {}

export type State = {
    -- The cloud's animation (#0x72's Param1).
    anim: number,
    -- The action its owner put it on in.
    action: Action?,
}

local CLOUD_SPRITE = asset.sprite("dust-cloud")
local DUSTMAN_SPRITE = asset.sprite("dustman")
local DUSTMAN_ANIM = 0x15

-- `sub_80E86AC`, `sub_80E87B8`: on its owner.
local function put_on(me: Object, sprite_asset: SpriteAsset, anim: number)
    me.pos = (me.related1 :: Object).pos
    local sprite = me.sprite
    sprite:load(sprite_asset)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E86F6`, `sub_80E8802`: on its owner, in his look, while he stays
-- in his action.
local function follow(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local owner = me.related1 :: Object
    me.pos = owner.pos
    me.visible = owner.visible
    local look, worn = me.sprite, owner.sprite
    look.palette = worn.palette
    look.color_shader = worn.color_shader
    look.white = worn.white
    look.mosaic = worn.mosaic
    me.flip = owner.flip
    look:set_flip(bit32.bxor(me.alliance, me.flip))
    look.alpha = worn.alpha
    if owner:navi_action() ~= (me.state :: State).action then
        me:set_lifecycle("destroy")
    end
end

-- The two kinds' update: its state's routine, then its sprite (stepping
-- even while paused, not while dimmed).
local function update(sprite_asset: SpriteAsset, fixed_anim: number?): (me: Object) -> ()
    return function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            put_on(me, sprite_asset, fixed_anim or (me.state :: State).anim)
        elseif l == "update" then
            follow(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_paused()
    end
end

clouds.cloud = define.kind {
    id = "dustman/cloud",
    pool = "effect",
    state = { anim = "u8", action = "action" },
    -- `sub_80E8794`.
    update = update(CLOUD_SPRITE, nil),
}

clouds.overlay = define.kind {
    id = "dustman/overlay",
    pool = "effect",
    state = { anim = "u8", action = "action" },
    -- `sub_80E8688`.
    update = update(DUSTMAN_SPRITE, DUSTMAN_ANIM),
}

local function spawn(kind: Kind, owner: Object, anim: number): Object?
    local o = battle.spawn(kind, owner.pos)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        local s = o.state :: State
        s.anim = anim
        s.action = owner:navi_action() :: Action
    end
    return o
end

-- `sub_80E887C`: a dust cloud on `owner` showing `anim`, in the action he
-- is running.
function clouds.put_cloud_on(owner: Object, anim: number): Object?
    return spawn(clouds.cloud, owner, anim)
end

-- `sub_80E8770`: his sprite's second copy on `owner`, in the action he is
-- running.
function clouds.put_overlay_on(owner: Object): Object?
    return spawn(clouds.overlay, owner, DUSTMAN_ANIM)
end

-- `sub_80E8894`, `sub_80E8788`: take one off (at its next update).
function clouds.take_off(o: Object)
    o:set_lifecycle("destroy")
end

return clouds
