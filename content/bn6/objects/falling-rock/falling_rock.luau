--!strict
-- A rock falling on a panel (attack object #0x1D, `sub_80C7F40`): the hit
-- of GroundCross's charged chip, which drops three (`sub_8012CB2`, in the
-- ruleset). It is spawned on its panel with its damage word; its first
-- update hits there (the hit spark and collision) and it goes. Its owner
-- (related1) is never set, so the owner's-action test reads memory the
-- game has no object at; a rock whose owner is in action 0xB would stay
-- on for 72 ticks (`sub_80C802C`), a branch no spawner reaches.

local falling_rock = {}

local SPRITE = "10-12"

-- Drawn this far toward its front, in whole pixels.
local AHEAD = 0x88

-- The owner action that keeps it on, and for how long (`byte_80C8068`, by
-- Param1).
local HOLD_ACTION = 0x0B
local HOLD_TICKS = { [0] = 0x48, [1] = 0x48, [2] = 0x48, [3] = 0x48 }

-- `sub_80C7F64`: on its panel at ground level (its Z's fraction left),
-- the sprite, and a hit registered on the panel (region 8, no hit
-- effect). False if no collision slot was free (it is freed).
local function init(me: Object): boolean
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD * 0x10000, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return false
    end
    me:setup_collision(4, 5, 3)
    me.collision.region = 8
    me.collision.hit_effect = 0xFF
    me.collision:present()
    me:set_lifecycle("update")
    return true
end

-- `sub_80C802C`: held on for its time, the hit ending once it landed.
local function hold(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = HOLD_TICKS[me:param(1)] or error("falling rock Param1 reads past its table (sub_80C802C)")
        return
    end
    if me.collision.hit_flags ~= 0 then
        me.collision.region = 0
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80C7FDC`: resolve the hit (and its spark); unless the owner is in
-- action 0xB, the rock goes.
local function update(me: Object)
    me.collision:remove()
    me:hit_spark()
    if not battle.over() then
        if me.collision.hit_flags ~= 0 then
            me.collision.region = 0
        end
        local owner = me.related1
        if owner and owner.action == HOLD_ACTION then
            if battle.dimmed() then
                return
            end
            hold(me)
            me.collision:present()
            return
        end
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

function falling_rock.update(me: Object)
    if me.lifecycle == "init" then
        if not init(me) then
            return
        end
    elseif me.lifecycle == "update" then
        update(me)
    else
        me:destroy()
        return
    end
    me:update_sprite_while_paused()
end

return falling_rock
