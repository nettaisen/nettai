--!strict
-- Django's coffin (BN5's attack object #0xBC, 0x080DE050; BN5's own): it stands over his target's panel, its look
-- the animation Django gives it (`coffin.set_anim`), until he lets it go (`coffin.remove`) or the battle ends.

local coffin = {}

export type State = {
    -- He let it go (+0x60).
    done: boolean,
}

local SPRITE = asset.sprite("bn5:django-coffin")

-- 0x080DE074: on its panel, on the ground, drawn with its sprite (animation 0, loaded with the sprite update).
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.visible = true
    me:set_lifecycle("update")
end

-- 0x080DE096: gone once he lets it go.
local function tick(me: Object)
    if (me.state :: State).done then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

coffin.kind = define.kind {
    id = "bn5:django/coffin",
    pool = "attack",
    state = { done = "bool" },
    -- 0x080DE050.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            tick(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080DE0AC: Django's coffin on (x, y).
function coffin.spawn(django: Object, x: number, y: number): Object?
    local o = battle.spawn(coffin.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = django.alliance, django.flip
        o.related1 = django
        o.run_while_dimmed = true
    end
    return o
end

-- 0x080DE0D6: its animation.
function coffin.set_anim(me: Object, anim: number)
    me.anim = anim
end

-- 0x080DE0D0: he lets it go.
function coffin.remove(me: Object)
    (me.state :: State).done = true
end

return coffin
