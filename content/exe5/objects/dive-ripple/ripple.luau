--!strict
-- The ripple over a body under the sea's surface (EXE5's effect object #0x3E, 0x080E4B64; EXE6's `sub_80E4CB8`, which
-- nothing starts): the navi's status tick keeps one while its flags say it is under (0x80000004, 0x0800DEB2: the role
-- `kinds.dive_ripple`, the body its first related) and ends it as it comes up. It sits on the surface over the body,
-- its animation by the body's (0 standing, 2 for the body's 3, 1 for the rest), shown on the consoles that see the
-- body (0x0800CDF4); as it starts, a splash (the effect table's row 0x5D, 14-1E).

local ripple = {}

local SPRITE = asset.sprite("ground-drill-effect")
local SPLASH = define.effect { sprite = asset.sprite("dive-splash"), anim = 0, palette = 0 }

-- 0x080E4BCA: over the body, on the surface.
local function follow(me: Object)
    local body = me.related1 :: Object
    local p = body.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.visible = true
    local a = body.anim
    me.anim = if a == 0 then 0 elseif a == 3 then 2 else 1
    me:hide_from_blind()
    if me.phase_init == 0 then
        battle.effect(me.pos, SPLASH)
        me.phase_init = 4
    end
end

ripple.kind = define.kind {
    id = "dive-ripple",
    pool = "effect",
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- 0x080E4B88: its sprite (0C-2D) with its shadow, all of it under the objects, animation 0. (Its call of
            -- 0x08002E8E, EXE6's `sub_800302A`, with 1 sets two bits of the sprite's +0x16 and its flag 0x40, which
            -- nothing that draws a sprite reads: its priority stays the field's.)
            me.visible = true
            local sprite = me.sprite
            sprite:load(SPRITE)
            sprite.shadow = "ground"
            sprite.under_objects = true
            me.anim, me.anim_loaded = 0, 0
            sprite:set_animation(0)
            sprite:step()
            me:set_lifecycle("update")
            follow(me)
        elseif l == "update" then
            follow(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

return ripple
