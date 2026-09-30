--!strict
-- A Reflector's counterattack (attack object #0x2F, `sub_80C9BC4`): a
-- wave that hits one panel at a time. Each segment stands on its panel
-- with its collision on, and after 2 ticks starts the next segment one
-- panel ahead, until one lands off the field; it stops hitting once it
-- hits something, and goes when its animation ends. See chips.md §3.3.

local reflected_shot = {}

local SPRITE = "14-04"
-- Ticks before the next segment starts.
local SPREAD_TICKS = 2

-- The actions: waiting to spread, then playing out.
local SPREADING, FADING = 0, 4

-- `sub_80C9CDA`: a segment on (x, y) at height `z` for `spawner`'s side,
-- dealing the damage word `damage`. Its spawn position is the spawner's
-- registers (panel y, garbage, z) and its parameters theirs too; its init
-- places it on its panel and reads neither.
function reflected_shot.spawn(spawner: Object, x: number, y: number, z: number, damage: number): Object?
    local o = battle.spawn_kind("reflected-shot", Vec3.new(y, 0, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = spawner.alliance, spawner.flip
        o.related1 = spawner
    end
    return o
end

-- `sub_80C9CA2`: after its wait, start the next segment ahead.
local function spread(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t ~= 0 then
        return
    end
    local damage = int.u32(me.damage + me.stamina * 0x10000)
    reflected_shot.spawn(me, me.panel_x + me.facing, me.panel_y, me.pos.z, damage)
    me.action = FADING
end

-- `sub_80C9CC4`: gone once its animation ends.
local function fade(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me.collision.region = 0
        me:set_lifecycle("destroy")
    end
end

-- `sub_80C9C5C`: resolve the last tick's hits; once it hit, no more.
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    if battle.over() then
        me.collision.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if me.collision.hit_flags ~= 0 then
        me.collision.region = 0
    end
    local a = me.action
    if a == SPREADING then
        spread(me)
    elseif a == FADING then
        fade(me)
    else
        error(string.format("reflected shot action %#x reads past its table", a))
    end
    me.collision:present()
end

-- `sub_80C9BE8`: off the field, nothing; else on its panel with its
-- collision on.
local function init(me: Object): boolean
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return false
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    me.timer = SPREAD_TICKS
    me.timer2 = 0
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return false
    end
    me:setup_collision(4, 5, 0)
    me.collision.region = 1
    me.collision.hit_effect = 0xFF
    me.collision:present()
    me:set_lifecycle("update")
    return true
end

-- `sub_80C9BC4`: the state, then the sprite.
function reflected_shot.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        if not init(me) then
            return
        end
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
        return
    end
    me:update_sprite()
end

return reflected_shot
