--!strict
-- DustMan's dust (effect objects #0x71 `sub_80E8688` and #0x72
-- `sub_80E8794`, the same routine with two looks): a sprite riding on its
-- owner, wearing his palette, look and flip, while the owner stays in an
-- action (#0x71: Param1; #0x72: Param2, its animation Param1). #0x72 is his
-- DustBrk's suction, #0x71 its blow. [unverified: no lab scenario reaches
-- it; docs/engine/standard-chips.md]

local dust_cloud = {}

-- The two kinds: effect index, sprite and animation (nil: Param1), and
-- which parameter holds the owner's action.
type Look = { sprite: SpriteId, anim: number?, action_param: number }
local LOOKS: { [number]: Look } = {
    [0x71] = { sprite = "08-0a", anim = 0x15, action_param = 1 },
    [0x72] = { sprite = "10-59", anim = nil, action_param = 2 },
}

-- `sub_80E8770` (#0x71) / `sub_80E887C` (#0x72): a cloud on `owner` with
-- `params`; its init puts it on the owner.
function dust_cloud.spawn(owner: Object, name: string, params: { number }): Object?
    local o = battle.spawn_kind(name, Vec3.zero, params)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80E8894` / `sub_80E8788`: end it.
function dust_cloud.stop(o: Object)
    o:set_lifecycle("destroy")
end

local function look_of(me: Object): Look
    local look = LOOKS[me.index]
    if look == nil then
        error(string.format("dust cloud as effect %#x", me.index))
    end
    return look
end

-- `sub_80E86F6` / `sub_80E8802`: follow the owner; end once it leaves
-- the action.
local function tick(me: Object)
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
    me.flip = owner.flip
    look:set_flip(bit32.bxor(me.alliance, me.flip))
    look.alpha = worn.alpha
    if owner.action ~= me:param(look_of(me).action_param) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E86AC` / `sub_80E87B8`.
local function init(me: Object)
    local l = look_of(me)
    me.pos = (me.related1 :: Object).pos
    local sprite = me.sprite
    sprite:load(l.sprite)
    sprite:set_animation(0)
    sprite.shadow = "with_sprite"
    me.visible = true
    local anim = l.anim or me:param(1)
    me.anim = anim
    me.anim_loaded = anim
    sprite:set_animation(anim)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80E8688` / `sub_80E8794`.
function dust_cloud.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:free()
        return
    end
    me:update_sprite_while_paused()
end

return dust_cloud
