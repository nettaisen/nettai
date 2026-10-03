--!strict
-- Django's sun (BN5's effect object #0x7B, 0x080E97F8; BN5's own): it hangs 64 pixels above him and 8 behind,
-- rising (animation 0) then shining (animation 1), brighter with Otenko's blessing (palette 3), until he puts it
-- away (`sun.remove`).

local sun = {}

export type State = {
    -- Otenko blesses Django's side (its Param1: palette 3 for 1).
    blessed: boolean,
    -- He put it away (+0x60).
    done: boolean,
}

local SPRITE = asset.sprite("bn5:django-sun")

-- 0x080E9840: its phase, then over him (he moves it).
local function tick(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        -- 0x080E9878: the timer (0) runs out at once: it shines.
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left <= 0 then
            me.anim = 1
            me.phase = 4
            me.phase_init = 0
        end
    elseif me.phase == 4 then
        -- 0x080E988A.
        if s.done then
            me.visible = false
            me:set_lifecycle("destroy")
        end
    else
        error(string.format("Django's sun phase %#x reads past its table (0x080E9870)", me.phase))
    end
    local owner = me.related1 :: Object
    local p = owner.pos
    me.pos = Vec3.new(int.i32(p.x + me.facing * -0x8_0000), p.y, int.i32(p.z + 0x40_0000))
end

-- 0x080E981C: rising (animation 0, loaded with the sprite update), drawn with its sprite.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0xFF
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    sprite.palette = if (me.state :: State).blessed then 3 else 0
    me:set_lifecycle("update")
    tick(me)
end

sun.kind = define.kind {
    id = "bn5:django/sun",
    pool = "effect",
    state = { blessed = "bool", done = "bool" },
    -- 0x080E97F8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080E989E: Django's sun.
function sun.spawn(django: Object, blessed: boolean): Object?
    local o = battle.spawn(sun.kind, Vec3.zero)
    if o then
        o.alliance, o.flip = django.alliance, django.flip
        o.related1 = django
        ;(o.state :: State).blessed = blessed
    end
    return o
end

-- 0x080E98B6: he puts it away.
function sun.remove(me: Object)
    (me.state :: State).done = true
end

return sun
